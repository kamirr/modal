pub mod node;
pub mod util;

use std::time::Duration;

use midly::MidiMessage;
use node::Node;
use serde::{Deserialize, Serialize};
use thunderdome::{Arena, Index};

use node::NodeEvent;

#[derive(Debug, Serialize, Deserialize)]
struct Entry {
    inputs: Vec<Option<OutputPort>>,
    node: Box<dyn Node>,
}

impl Clone for Entry {
    fn clone(&self) -> Self {
        Entry {
            inputs: self.inputs.clone(),
            node: dyn_clone::clone_box(&*self.node),
        }
    }
}

impl Entry {
    fn new(inputs: Vec<Option<OutputPort>>, node: Box<dyn Node>) -> Self {
        Entry { inputs, node }
    }
}

#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize, strum::EnumDiscriminants)]
#[strum_discriminants(name(ValueKind))]
#[strum_discriminants(vis(pub))]
pub enum Value {
    #[default]
    None,
    Disconnected,
    #[serde(skip)]
    Midi {
        channel: u8,
        message: MidiMessage,
    },
    Float(f32),
    FloatArray(Vec<f32>),
    Beat(Duration),
}

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub struct OutputPort {
    #[serde(with = "crate::util::serde_thunderdome_index")]
    pub node: Index,
    pub port: usize,
}

impl OutputPort {
    pub fn new(node: Index, port: usize) -> Self {
        OutputPort { node, port }
    }
}

pub struct Output {
    pub name: String,
    pub kind: ValueKind,
}

impl Output {
    pub fn new(name: impl Into<String>, kind: ValueKind) -> Self {
        Output {
            name: name.into(),
            kind,
        }
    }
}

impl Value {
    pub fn as_midi(&self) -> Option<(u8, &MidiMessage)> {
        match self {
            Value::Midi { channel, message } => Some((*channel, message)),
            _ => None,
        }
    }

    pub fn as_float(&self) -> Option<f32> {
        match self {
            Value::Float(s) => Some(*s),
            _ => None,
        }
    }

    pub fn as_float_array(&self) -> Option<Vec<f32>> {
        match self {
            Value::Float(s) => Some(vec![*s]),
            Value::FloatArray(s) => Some(s.clone()),
            _ => None,
        }
    }

    pub fn as_beat(&self) -> Option<Duration> {
        match self {
            Value::Beat(dur) => Some(*dur),
            _ => None,
        }
    }

    pub fn disconnected(&self) -> bool {
        self == &Value::Disconnected
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Runtime {
    #[serde(skip)]
    values: Vec<Vec<Value>>,
    #[serde(with = "crate::util::serde_arena")]
    nodes: Arena<Entry>,
}

impl Runtime {
    pub fn new() -> Self {
        Runtime {
            values: Vec::new(),
            nodes: Arena::new(),
        }
    }

    pub fn insert(
        &mut self,
        inputs: impl Into<Vec<Option<OutputPort>>>,
        node: Box<dyn Node + 'static>,
    ) -> Index {
        let inputs = inputs.into();
        assert_eq!(inputs.len(), node.inputs().len());
        self.nodes.insert(Entry::new(inputs, node))
    }

    pub fn remove(&mut self, index: Index) {
        self.nodes.remove(index);
        for (_, entry) in &mut self.nodes {
            for input in &mut entry.inputs {
                if let Some(port) = *input {
                    if port.node == index {
                        *input = None;
                    }
                }
            }
        }
    }

    pub fn set_input(&mut self, index: Index, port: usize, new_input: Option<OutputPort>) {
        self.nodes[index].inputs[port] = new_input;
    }

    pub fn set_all_inputs(&mut self, index: Index, new_inputs: Vec<Option<OutputPort>>) {
        self.nodes[index].inputs = new_inputs;
    }

    pub fn step(&mut self) -> Vec<(Index, Vec<NodeEvent>)> {
        let mut evs = Vec::new();
        let mut buf = Vec::new();

        self.values.clear();

        for (idx, entry) in &self.nodes {
            while self.values.len() <= idx.slot() as usize {
                self.values.push(Vec::default());
            }

            let idx = idx.slot() as usize;
            let target_len = entry.node.output().len();

            if self.values[idx].len() != target_len {
                self.values[idx] = vec![Value::None; target_len];
            }

            entry.node.read(&mut self.values[idx]);
        }

        for (idx, entry) in &mut self.nodes {
            buf.clear();
            for input in &mut entry.inputs {
                buf.push(match input {
                    Some(input) => self.values[input.node.slot() as usize][input.port].clone(),
                    None => Value::Disconnected,
                });
            }

            let evs_one = entry.node.feed(&buf);
            evs.push((idx, evs_one));
        }

        evs
    }

    pub fn peek(&self, input: OutputPort) -> Value {
        self.values
            .get(input.node.slot() as usize)
            .map(|vec| vec[input.port].clone())
            .unwrap_or(Value::None)
    }

    pub fn nodes(&self) -> impl Iterator<Item = (Index, &Box<dyn Node>)> {
        self.nodes.iter().map(|(idx, entry)| (idx, &entry.node))
    }
}
