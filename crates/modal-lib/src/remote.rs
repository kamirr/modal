use std::{
    collections::HashMap,
    sync::mpsc::{channel, Receiver, Sender, TryRecvError},
    time::Duration,
};

use bimap::BiHashMap;
use egui_graph_edit::NodeId;
use thunderdome::Index;

use runtime::{
    node::{Node, NodeEvent},
    OutputPort, Runtime, Value,
};

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
    Play(Option<OutputPort>),
    Record(Index, usize),
    StopRecording(Index, usize),
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
    fn feed(&mut self, samples: &[f32]);
    fn start(&mut self);
}

pub struct RuntimeRemote {
    tx: Sender<RtRequest>,
    rx: Receiver<RtResponse>,
    must_wait: bool,
    mapping: BiHashMap<NodeId, Index>,
    recordings: HashMap<OutputPort, Vec<Value>>,
    node_events: Vec<(Index, Vec<NodeEvent>)>,
    runtime: Option<Runtime>,
}

impl RuntimeRemote {
    pub fn from_parts(
        mut rt: Runtime,
        mapping: Vec<(NodeId, u64)>,
        mut audio_out: Box<dyn AudioOut>,
    ) -> Self {
        let (cmd_tx, cmd_rx) = channel();
        let (resp_tx, resp_rx) = channel();

        let mut record = None;
        let buf_size = 512;
        let mut buf = vec![0.0; buf_size];

        while audio_out.queue_len() as f32 * buf_size as f32 / 44100.0 < 0.1 {
            audio_out.feed(&buf);
        }
        audio_out.start();

        let mut recording = HashMap::<OutputPort, Vec<Value>>::new();

        std::thread::spawn(move || {
            loop {
                while audio_out.queue_len() as f32 * buf_size as f32 / 44100.0 > 0.08 {
                    std::thread::sleep(Duration::from_millis(10));
                }

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

                    audio_out.feed(&buf);
                }

                for (input, buffer) in &mut recording {
                    if !buffer.is_empty() {
                        resp_tx
                            .send(RtResponse::Samples(*input, std::mem::take(buffer)))
                            .ok();
                    }
                }

                let cmd = match cmd_rx.try_recv() {
                    Ok(cmd) => cmd,
                    Err(TryRecvError::Empty) => continue,
                    Err(TryRecvError::Disconnected) => break,
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
                }

                resp_tx.send(RtResponse::Step).ok();
            }

            println!("Runtime stopped");
        });

        RuntimeRemote {
            tx: cmd_tx,
            rx: resp_rx,
            must_wait: false,
            mapping: mapping
                .into_iter()
                .map(|(id, bits)| (id, Index::from_bits(bits).unwrap()))
                .collect(),
            recordings: HashMap::new(),
            node_events: Vec::new(),
            runtime: None,
        }
    }

    pub fn start(audio_out: Box<dyn AudioOut>) -> Self {
        Self::from_parts(Runtime::new(), Vec::new(), audio_out)
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

    pub fn shutdown(&mut self) {
        self.tx.send(RtRequest::Shutdown).ok();
    }

    pub fn process(&mut self, resp: RtResponse) {
        match resp {
            RtResponse::Inserted(id, idx) => {
                self.mapping.insert(id, idx);
            }
            RtResponse::NodeEvents(evs) => {
                self.node_events.extend(evs.into_iter());
            }
            RtResponse::RuntimeCloned(runtime) => {
                self.runtime = Some(runtime);
            }
            RtResponse::Samples(index, samples) => {
                self.recordings
                    .entry(index)
                    .or_default()
                    .extend(samples.into_iter());
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
