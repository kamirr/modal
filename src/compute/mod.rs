pub mod node;

use node::{basic::Placeholder, Node};
use thunderdome::{Arena, Index};

use self::node::{Param, ParamSignature};

struct Entry {
    inputs: Vec<Index>,
    node: Box<dyn Node>,
}

impl Entry {
    fn new(inputs: Vec<Index>, node: Box<dyn Node>) -> Self {
        Entry {
            inputs: inputs.into(),
            node,
        }
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

    pub fn insert(&mut self, inputs: impl Into<Vec<Index>>, node: impl Node + 'static) -> Index {
        self.insert_box(inputs, Box::new(node))
    }

    pub fn insert_box(
        &mut self,
        inputs: impl Into<Vec<Index>>,
        node: Box<dyn Node + 'static>,
    ) -> Index {
        self.nodes.insert(Entry::new(inputs.into(), node))
    }

    pub fn reserve(&mut self) -> Index {
        self.nodes.insert(Entry::new(vec![], Box::new(Placeholder)))
    }

    pub fn insert_at(
        &mut self,
        index: Index,
        inputs: impl Into<Vec<Index>>,
        node: impl Node + 'static,
    ) {
        self.insert_box_at(index, inputs, Box::new(node));
    }

    pub fn insert_box_at(
        &mut self,
        index: Index,
        inputs: impl Into<Vec<Index>>,
        node: Box<dyn Node + 'static>,
    ) {
        self.nodes[index] = Entry::new(inputs.into(), node);
    }

    pub fn remove(&mut self, index: Index) {
        self.nodes.remove(index);
    }

    pub fn set_input(&mut self, index: Index, port: usize, new_input: Index) {
        self.nodes[index].inputs[port] = new_input;
    }

    pub fn set_param(&mut self, index: Index, param: Vec<Param>) {
        let node = &mut self.nodes[index].node;
        node.set_param(param.as_slice());
    }

    pub fn get_param(&mut self, index: Index) -> Vec<(String, ParamSignature, Param)> {
        let node = &mut self.nodes[index].node;

        node.meta()
            .params
            .into_iter()
            .zip(node.get_param().into_iter())
            .map(|((name, sig), param)| (name, sig, param))
            .collect()
    }

    pub fn step(&mut self) {
        let mut buf = Vec::new();

        self.values.clear();

        for (idx, entry) in &self.nodes {
            self.values.insert_at_slot(idx.slot(), entry.node.read());
        }

        for (_idx, entry) in &mut self.nodes {
            buf.clear();
            for &mut input in &mut entry.inputs {
                buf.push(*self.values.get_by_slot(input.slot()).unwrap().1);
            }

            entry.node.feed(&buf);
        }
    }

    pub fn peek(&self, index: Index) -> f32 {
        *self.values.get_by_slot(index.slot()).unwrap().1
    }
}
