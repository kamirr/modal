use std::sync::mpsc::{channel, Receiver, Sender, TryRecvError};

use bimap::BiHashMap;
use egui_node_graph::NodeId;
use thunderdome::Index;

use crate::compute::{
    node::{Node, NodeEvent},
    Runtime,
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
}

pub enum RtResponse {
    Inserted(NodeId, Index),
    NodeEvents(Vec<(Index, Vec<NodeEvent>)>),
    Step,
}

pub struct RuntimeRemote {
    tx: Sender<RtRequest>,
    rx: Receiver<RtResponse>,
    must_wait: bool,
    mapping: BiHashMap<NodeId, Index>,
    node_events: Vec<(Index, Vec<NodeEvent>)>,
}

impl RuntimeRemote {
    pub fn start() -> Self {
        let (cmd_tx, cmd_rx) = channel();
        let (resp_tx, resp_rx) = channel();

        let mut rt = Runtime::new();

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

        std::thread::spawn(move || loop {
            while sink.len() as f32 * buf_size as f32 / 44100.0 < 0.1 {
                if let Some(record) = record {
                    for s in &mut buf {
                        let evs = rt.step();
                        if !evs.is_empty() {
                            resp_tx.send(RtResponse::NodeEvents(evs)).ok();
                        }

                        *s = rt.peek(record);
                    }
                } else {
                    for s in &mut buf {
                        let evs = rt.step();
                        if !evs.is_empty() {
                            resp_tx.send(RtResponse::NodeEvents(evs)).ok();
                        }

                        *s = 0.0;
                    }
                }

                let source = rodio::buffer::SamplesBuffer::new(1, 44100, buf.clone());
                sink.append(source);
            }

            let cmd = match cmd_rx.try_recv() {
                Ok(cmd) => cmd,
                Err(TryRecvError::Empty) => continue,
                Err(TryRecvError::Disconnected) => return,
            };

            match cmd {
                RtRequest::Insert { id, inputs, node } => {
                    let idx = rt.insert(inputs, node);
                    resp_tx.send(RtResponse::Inserted(id, idx)).ok();
                }
                RtRequest::Play(node) => {
                    record = node;
                    eprintln!("playback from {record:?}");
                }
                RtRequest::SetInput { src, dst, port } => {
                    rt.set_input(dst, port, src);
                }
                RtRequest::SetAllInputs { dst, inputs } => {
                    rt.set_all_inputs(dst, inputs);
                }
                RtRequest::Remove(index) => {
                    rt.remove(index);
                }
            }

            resp_tx.send(RtResponse::Step).ok();
        });

        RuntimeRemote {
            tx: cmd_tx,
            rx: resp_rx,
            must_wait: false,
            mapping: BiHashMap::new(),
            node_events: Vec::new(),
        }
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

    pub fn record(&mut self, id: Option<NodeId>) {
        let idx = id.and_then(|id| self.mapping.get_by_left(&id).cloned());
        self.tx.send(RtRequest::Play(idx)).ok();
    }

    pub fn process(&mut self, resp: RtResponse) {
        match resp {
            RtResponse::Inserted(id, idx) => {
                self.mapping.insert(id, idx);
            }
            RtResponse::NodeEvents(evs) => {
                self.node_events.extend(evs.into_iter());
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
}

impl Default for RuntimeRemote {
    fn default() -> Self {
        RuntimeRemote::start()
    }
}
