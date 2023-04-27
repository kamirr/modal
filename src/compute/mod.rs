pub mod node;

use midly::MidiMessage;
use node::Node;
use serde::{Deserialize, Serialize};
use thunderdome::{Arena, Index};

use self::node::NodeEvent;

#[derive(Debug, Serialize, Deserialize)]
struct Entry {
    #[serde(with = "crate::util::serde_vec_opt_idx")]
    inputs: Vec<Option<Index>>,
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
    fn new(inputs: Vec<Option<Index>>, node: Box<dyn Node>) -> Self {
        Entry { inputs, node }
    }
}

#[derive(Debug, Clone)]
pub enum Value {
    None,
    Midi(MidiMessage),
    Float(f32),
    FloatArray(Vec<f32>),
}

impl Value {
    pub fn as_midi(&self) -> Option<&MidiMessage> {
        match self {
            Value::Midi(msg) => Some(msg),
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
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Runtime {
    #[serde(skip)]
    values: Vec<Value>,
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
        inputs: impl Into<Vec<Option<Index>>>,
        node: Box<dyn Node + 'static>,
    ) -> Index {
        let inputs = inputs.into();
        assert_eq!(inputs.len(), node.inputs().len());
        self.nodes.insert(Entry::new(inputs, node))
    }

    pub fn remove(&mut self, index: Index) {
        self.nodes.remove(index);
    }

    pub fn set_input(&mut self, index: Index, port: usize, new_input: Option<Index>) {
        self.nodes[index].inputs[port] = new_input;
    }

    pub fn set_all_inputs(&mut self, index: Index, new_inputs: Vec<Option<Index>>) {
        self.nodes[index].inputs = new_inputs;
    }

    pub fn step(&mut self) -> Vec<(Index, Vec<NodeEvent>)> {
        let mut evs = Vec::new();
        let mut buf = Vec::new();

        self.values.clear();

        for (idx, entry) in &self.nodes {
            while self.values.len() <= idx.slot() as usize {
                self.values.push(Value::None);
            }
            self.values[idx.slot() as usize] = entry.node.read();
        }

        for (idx, entry) in &mut self.nodes {
            buf.clear();
            for &mut input in &mut entry.inputs {
                buf.push(match input {
                    Some(in_index) => self.values[in_index.slot() as usize].clone(),
                    None => Value::None,
                });
            }

            let evs_one = entry.node.feed(&buf);
            evs.push((idx, evs_one));
        }

        evs
    }

    pub fn peek(&self, index: Index) -> Value {
        self.values
            .get(index.slot() as usize)
            .cloned()
            .unwrap_or(Value::None)
    }

    pub fn nodes(&self) -> impl Iterator<Item = (Index, &Box<dyn Node>)> {
        self.nodes.iter().map(|(idx, entry)| (idx, &entry.node))
    }
}
