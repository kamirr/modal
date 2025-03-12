pub mod rodio_out;
pub mod stream_audio_out;

use std::{
    collections::HashMap,
    sync::mpsc::{channel, Receiver, Sender, TryRecvError},
    time::Duration,
};

use bimap::BiHashMap;
use egui_graph_edit::NodeId;
use serde::{Deserialize, Serialize};
use strum::Display;
use thunderdome::Index;

use runtime::{
    node::{Node, NodeEvent},
    OutputPort, Runtime, Value, ValueKind,
};

#[derive(Clone, Debug, Display, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ExternInput {
    TrackAudio,
    Midi,
}

#[derive(Debug)]
pub enum RtRequest {
    Insert {
        id: NodeId,
        inputs: Vec<Option<OutputPort>>,
        node: Box<dyn Node>,
    },
    Remove(Index),
    SetInput {
        src: Option<OutputPort>,
        dst: Index,
        port: usize,
    },
    SetAllInputs {
        dst: Index,
        inputs: Vec<Option<OutputPort>>,
    },
    ExternDefine {
        input: ExternInput,
        kind: ValueKind,
    },
    ExternAppend {
        input: ExternInput,
        values: Vec<Value>,
    },
    Play(Option<OutputPort>),
    Record(Index, usize),
    StopRecording(Index, usize),
    ReplaceRuntime(Runtime),
    CloneRuntime,
    Shutdown,
}

pub enum RtResponse {
    Inserted(NodeId, Index),
    NodeEvents(Vec<(Index, Vec<NodeEvent>)>),
    RuntimeCloned(Runtime),
    Samples(OutputPort, Vec<Value>),
    Step,
}

pub trait AudioOut: Send {
    fn queue_len(&self) -> usize;
    #[must_use]
    fn feed(&mut self, samples: &[f32]) -> bool;
    fn start(&mut self);
}

pub struct RuntimeRemote {
    pub tx: Sender<RtRequest>,
    rx: Receiver<RtResponse>,
    must_wait: bool,
    mapping: BiHashMap<NodeId, Index>,
    recordings: HashMap<OutputPort, Vec<Value>>,
    node_events: Vec<(Index, Vec<NodeEvent>)>,
    runtime: Option<Runtime>,
}

impl RuntimeRemote {
    pub fn new() -> (Self, Sender<RtResponse>, Receiver<RtRequest>) {
        let (cmd_tx, cmd_rx) = channel();
        let (resp_tx, resp_rx) = channel();
        let this = RuntimeRemote {
            tx: cmd_tx,
            rx: resp_rx,
            must_wait: false,
            mapping: BiHashMap::new(),
            recordings: HashMap::new(),
            node_events: Vec::new(),
            runtime: None,
        };

        (this, resp_tx, cmd_rx)
    }

    pub fn start(mut audio_out: Box<dyn AudioOut>) -> Self {
        println!("Runtime init");

        let (this, resp_tx, cmd_rx) = RuntimeRemote::new();
        let mut rt = Runtime::new();

        let mut extern_input_indices = HashMap::new();

        let mut record = None;
        let buf_size = 512;
        let mut buf = vec![0.0; buf_size];

        while audio_out.queue_len() as f32 * buf_size as f32 / 44100.0 < 0.01 {
            let _ = audio_out.feed(&buf);
        }
        audio_out.start();

        let mut recording = HashMap::<OutputPort, Vec<Value>>::new();

        std::thread::spawn(move || {
            println!("Runtime thread started");

            'outer: loop {
                let emit_output = audio_out.queue_len() as f32 * buf_size as f32 / 44100.0 < 0.08;
                if !emit_output {
                    std::thread::sleep(Duration::from_millis(10));
                }

                if emit_output {
                    while audio_out.queue_len() as f32 * buf_size as f32 / 44100.0 < 0.1 {
                        for s in &mut buf {
                            let evs = rt.step();
                            if !evs.is_empty() {
                                resp_tx.send(RtResponse::NodeEvents(evs)).ok();
                            }

                            *s = record
                                .map(|idx| rt.peek(idx))
                                .as_ref()
                                .and_then(Value::as_float)
                                .unwrap_or_default();

                            for (input, buffer) in &mut recording {
                                let value = rt.peek(*input);
                                buffer.push(value);
                            }
                        }

                        let sink_ok = audio_out.feed(&buf);
                        if !sink_ok {
                            break 'outer;
                        }
                    }

                    for (input, buffer) in &mut recording {
                        if !buffer.is_empty() {
                            resp_tx
                                .send(RtResponse::Samples(*input, std::mem::take(buffer)))
                                .ok();
                        }
                    }
                }

                loop {
                    let cmd = match cmd_rx.try_recv() {
                        Ok(cmd) => cmd,
                        Err(TryRecvError::Empty) => break,
                        Err(TryRecvError::Disconnected) => break 'outer,
                    };

                    match cmd {
                        RtRequest::Insert { id, inputs, node } => {
                            let idx = rt.insert(inputs, node);
                            resp_tx.send(RtResponse::Inserted(id, idx)).ok();
                        }
                        RtRequest::Remove(index) => {
                            rt.remove(index);
                            recording.retain(|rec, _| rec.node != index);

                            if let Some(OutputPort { node, .. }) = record {
                                if node == index {
                                    record = None;
                                }
                            }
                        }
                        RtRequest::SetInput { src, dst, port } => {
                            rt.set_input(dst, port, src);
                        }
                        RtRequest::SetAllInputs { dst, inputs } => {
                            rt.set_all_inputs(dst, inputs);
                        }
                        RtRequest::ExternDefine { input, kind } => {
                            let index = rt.extern_inputs().define(input.to_string(), kind);
                            extern_input_indices.insert(input, index);
                        }
                        RtRequest::ExternAppend { input, values } => {
                            let index = extern_input_indices[&input];
                            rt.extern_inputs().extend(index, values.into_iter());
                        }
                        RtRequest::Play(node) => {
                            record = node;
                        }
                        RtRequest::Record(index, port) => {
                            recording.insert(OutputPort::new(index, port), Vec::new());
                        }
                        RtRequest::StopRecording(index, port) => {
                            recording.remove(&OutputPort::new(index, port));
                        }
                        RtRequest::CloneRuntime => {
                            resp_tx.send(RtResponse::RuntimeCloned(rt.clone())).ok();
                        }
                        RtRequest::Shutdown => {
                            break;
                        }
                        RtRequest::ReplaceRuntime(runtime) => {
                            rt = runtime;
                        }
                    }
                }

                resp_tx.send(RtResponse::Step).ok();
            }

            println!("Runtime stopped");
        });

        this
    }

    pub fn insert(&mut self, id: NodeId, node: Box<dyn Node>) {
        let inputs = vec![None; node.inputs().len()];
        self.tx.send(RtRequest::Insert { id, inputs, node }).ok();
        self.must_wait = true;
    }

    pub fn remove(&mut self, id: NodeId) {
        let idx = self.mapping.get_by_left(&id).cloned().unwrap();
        self.tx.send(RtRequest::Remove(idx)).ok();
        self.mapping.remove_by_left(&id);
        self.must_wait = true;
    }

    pub fn set_inputs(&mut self, dst: NodeId, inputs: Vec<Option<OutputPort>>) {
        self.tx
            .send(RtRequest::SetAllInputs {
                dst: *self.mapping.get_by_left(&dst).unwrap(),
                inputs,
            })
            .ok();
    }

    pub fn connect(&mut self, src: NodeId, src_port: usize, dst: NodeId, dst_port: usize) {
        let src = self.mapping.get_by_left(&src).cloned().unwrap();
        let dst = self.mapping.get_by_left(&dst).cloned().unwrap();
        self.tx
            .send(RtRequest::SetInput {
                src: Some(OutputPort::new(src, src_port)),
                dst,
                port: dst_port,
            })
            .ok();
    }

    pub fn disconnect(&mut self, dst: NodeId, port: usize) {
        let dst = self.mapping.get_by_left(&dst).cloned().unwrap();
        self.tx
            .send(RtRequest::SetInput {
                src: None,
                dst,
                port,
            })
            .ok();
    }

    pub fn play(&mut self, id: Option<(NodeId, usize)>) {
        let input = id.and_then(|(id, port)| {
            self.mapping
                .get_by_left(&id)
                .cloned()
                .map(|idx| OutputPort::new(idx, port))
        });
        self.tx.send(RtRequest::Play(input)).ok();
    }

    pub fn record(&mut self, id: NodeId, port: usize) {
        let idx = *self.mapping.get_by_left(&id).unwrap();
        self.tx.send(RtRequest::Record(idx, port)).ok();
    }

    pub fn stop_recording(&mut self, id: NodeId, port: usize) {
        let idx = *self.mapping.get_by_left(&id).unwrap();
        self.tx.send(RtRequest::StopRecording(idx, port)).ok();
    }

    pub fn replace_runtime(&mut self, rt: Runtime, mapping: Vec<(NodeId, u64)>) {
        self.mapping = mapping
            .into_iter()
            .map(|(id, bits)| (id, Index::from_bits(bits).unwrap()))
            .collect();
        self.tx.send(RtRequest::ReplaceRuntime(rt)).ok();
    }

    pub fn shutdown(&mut self) {
        self.tx.send(RtRequest::Shutdown).ok();
    }

    pub fn process(&mut self, resp: RtResponse) {
        match resp {
            RtResponse::Inserted(id, idx) => {
                self.mapping.insert(id, idx);
            }
            RtResponse::NodeEvents(evs) => {
                self.node_events.extend(evs);
            }
            RtResponse::RuntimeCloned(runtime) => {
                self.runtime = Some(runtime);
            }
            RtResponse::Samples(index, samples) => {
                self.recordings.entry(index).or_default().extend(samples);
            }
            RtResponse::Step => {}
        }
    }

    pub fn events(&mut self) -> Vec<(Index, Vec<NodeEvent>)> {
        std::mem::take(&mut self.node_events)
    }

    pub fn wait(&mut self) {
        if self.must_wait {
            while let Ok(resp) = self.rx.recv() {
                if let RtResponse::Step = &resp {
                    break;
                }

                self.process(resp);
            }

            self.must_wait = false;
        }

        while let Ok(resp) = self.rx.try_recv() {
            self.process(resp);
        }
    }

    pub fn id_to_index(&self, id: NodeId) -> Option<Index> {
        self.mapping.get_by_left(&id).copied()
    }

    pub fn index_to_id(&self, idx: Index) -> Option<NodeId> {
        self.mapping.get_by_right(&idx).copied()
    }

    pub fn save_state(&mut self) -> (Runtime, Vec<(NodeId, u64)>) {
        self.tx.send(RtRequest::CloneRuntime).ok();
        loop {
            if let Some(rt) = self.runtime.take() {
                let mapping = self
                    .mapping
                    .iter()
                    .map(|(node_id, index)| (*node_id, index.to_bits()))
                    .collect();
                return (rt, mapping);
            }

            self.must_wait = true;
            self.wait();
        }
    }

    pub fn recordings(&mut self) -> Vec<(OutputPort, Vec<Value>)> {
        self.recordings
            .iter_mut()
            .filter(|(_, buf)| !buf.is_empty())
            .map(|(k, v)| (*k, std::mem::take(v)))
            .collect()
    }
}
