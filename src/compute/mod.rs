pub mod node;

use node::Node;
use thunderdome::{Arena, Index};

use self::node::NodeEvent;

struct Entry {
    inputs: Vec<Option<Index>>,
    node: Box<dyn Node>,
}

impl Entry {
    fn new(inputs: Vec<Option<Index>>, node: Box<dyn Node>) -> Self {
        Entry { inputs, node }
    }
}

pub struct Runtime {
    values: Arena<f32>,
    nodes: Arena<Entry>,
}

impl Runtime {
    pub fn new() -> Self {
        Runtime {
            values: Arena::new(),
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
            self.values.insert_at_slot(idx.slot(), entry.node.read());
        }

        for (idx, entry) in &mut self.nodes {
            buf.clear();
            for &mut input in &mut entry.inputs {
                buf.push(match input {
                    Some(in_index) => Some(*self.values.get_by_slot(in_index.slot()).unwrap().1),
                    None => None,
                });
            }

            let evs_one = entry.node.feed(&buf);
            evs.push((idx, evs_one));
        }

        evs
    }

    pub fn peek(&self, index: Index) -> f32 {
        self.values
            .get_by_slot(index.slot())
            .map(|(_idx, val)| *val)
            .unwrap_or(0.0)
    }
}
