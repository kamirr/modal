use std::{
    collections::HashMap,
    sync::mpsc::{channel, Receiver, Sender, TryRecvError},
    time::Duration,
};

use bimap::BiHashMap;
use egui_node_graph::NodeId;
use thunderdome::Index;

use crate::compute::{
    node::{Node, NodeEvent},
    Runtime, Value,
};

#[derive(Debug)]
pub enum RtRequest {
    Insert {
        id: NodeId,
        inputs: Vec<Option<Index>>,
        node: Box<dyn Node>,
    },
    Remove(Index),
    SetInput {
        src: Option<Index>,
        dst: Index,
        port: usize,
    },
    SetAllInputs {
        dst: Index,
        inputs: Vec<Option<Index>>,
    },
    Play(Option<Index>),
    Record(Index),
    StopRecording(Index),
    CloneRuntime,
    Shutdown,
}

pub enum RtResponse {
    Inserted(NodeId, Index),
    NodeEvents(Vec<(Index, Vec<NodeEvent>)>),
    RuntimeCloned(Runtime),
    Samples(Index, Vec<Value>),
    Step,
}

pub struct RuntimeRemote {
    tx: Sender<RtRequest>,
    rx: Receiver<RtResponse>,
    must_wait: bool,
    mapping: BiHashMap<NodeId, Index>,
    recordings: HashMap<NodeId, Vec<Value>>,
    node_events: Vec<(Index, Vec<NodeEvent>)>,
    runtime: Option<Runtime>,
}

impl RuntimeRemote {
    pub fn with_rt_and_mapping(mut rt: Runtime, mapping: Vec<(NodeId, u64)>) -> Self {
        let (cmd_tx, cmd_rx) = channel();
        let (resp_tx, resp_rx) = channel();

        let mut record = None;
        let buf_size = 512;
        let mut buf = vec![0.0; buf_size];

        let (stream, handle) = rodio::OutputStream::try_default().unwrap();
        std::mem::forget(stream);

        let sink = rodio::Sink::try_new(&handle).unwrap();
        while sink.len() as f32 * buf_size as f32 / 44100.0 < 0.1 {
            let source = rodio::buffer::SamplesBuffer::new(1, 44100, buf.clone());
            sink.append(source);
        }
        sink.play();

        let mut recording = HashMap::<Index, Vec<Value>>::new();

        std::thread::spawn(move || {
            loop {
                while sink.len() as f32 * buf_size as f32 / 44100.0 > 0.08 {
                    std::thread::sleep(Duration::from_millis(10));
                }

                while sink.len() as f32 * buf_size as f32 / 44100.0 < 0.1 {
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

                        for (idx, buffer) in &mut recording {
                            buffer.push(rt.peek(*idx));
                        }
                    }

                    let source = rodio::buffer::SamplesBuffer::new(1, 44100, buf.clone());
                    sink.append(source);
                }

                for (idx, buffer) in &mut recording {
                    if !buffer.is_empty() {
                        resp_tx
                            .send(RtResponse::Samples(*idx, std::mem::take(buffer)))
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
                        recording.remove(&index);
                        if record == Some(index) {
                            record = None;
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
                    RtRequest::Record(index) => {
                        recording.insert(index, Vec::new());
                    }
                    RtRequest::StopRecording(index) => {
                        recording.remove(&index);
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

    pub fn start() -> Self {
        Self::with_rt_and_mapping(Runtime::new(), Vec::new())
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

    pub fn set_inputs(&mut self, dst: NodeId, inputs: Vec<Option<Index>>) {
        self.tx
            .send(RtRequest::SetAllInputs {
                dst: *self.mapping.get_by_left(&dst).unwrap(),
                inputs,
            })
            .ok();
    }

    pub fn connect(&mut self, src: NodeId, dst: NodeId, port: usize) {
        let src = self.mapping.get_by_left(&src).cloned().unwrap();
        let dst = self.mapping.get_by_left(&dst).cloned().unwrap();
        self.tx
            .send(RtRequest::SetInput {
                src: Some(src),
                dst,
                port,
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

    pub fn play(&mut self, id: Option<NodeId>) {
        let idx = id.and_then(|id| self.mapping.get_by_left(&id).cloned());
        self.tx.send(RtRequest::Play(idx)).ok();
    }

    pub fn record(&mut self, id: NodeId) {
        let idx = *self.mapping.get_by_left(&id).unwrap();
        self.tx.send(RtRequest::Record(idx)).ok();
    }

    pub fn stop_recording(&mut self, id: NodeId) {
        let idx = *self.mapping.get_by_left(&id).unwrap();
        self.tx.send(RtRequest::StopRecording(idx)).ok();
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
                let Some(&node_id) = self.mapping.get_by_right(&index) else {
                    return;
                };
                self.recordings
                    .entry(node_id)
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

    pub fn recordings(&mut self) -> Vec<(NodeId, Vec<Value>)> {
        self.recordings
            .iter_mut()
            .filter(|(_, buf)| !buf.is_empty())
            .map(|(k, v)| (*k, std::mem::take(v)))
            .collect()
    }
}

impl Default for RuntimeRemote {
    fn default() -> Self {
        RuntimeRemote::start()
    }
}
