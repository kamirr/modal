mod dgraph;
mod node;

use node::{all::*, Node};

use thunderdome::{Arena, Index};
use wav::WAV_FORMAT_IEEE_FLOAT;

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

struct Runtime {
    nodes: Arena<Entry>,
}

impl Runtime {
    pub fn new() -> Self {
        Runtime {
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

    pub fn step(&mut self, peek: Index) -> f32 {
        let mut values = Arena::new();
        let mut buf = Vec::new();

        for (idx, entry) in &self.nodes {
            values.insert_at_slot(idx.slot(), entry.node.read());
        }

        for (_idx, entry) in &mut self.nodes {
            buf.clear();
            for &mut input in &mut entry.inputs {
                buf.push(*values.get_by_slot(input.slot()).unwrap().1);
            }

            entry.node.feed(&buf);
        }

        values[peek]
    }
}

fn noise(len: usize) -> impl Node {
    let mut delay = delay(len);
    delay.apply(|data| {
        for f in data {
            *f = rand::random::<f32>() * 0.3;
        }
    });

    chain2(constant(0.0), delay)
}

fn main() {
    let mut rt = Runtime::new();

    let net = feedback_many(
        (
            chain2(delay(44100 / 440), fir([0.5, 0.5])),
            chain2(delay(44100 / 660), fir([0.5, 0.5])),
        ),
        (dot([0.99, 0.01]), dot([0.01, 0.99])),
        add(),
    );

    let in1 = rt.insert([], noise(100));
    let in2 = rt.insert([], constant(0.0));
    let net_out = rt.insert([in1, in2], net);

    let data: Vec<_> = (0..44100 * 2).map(|_| rt.step(net_out)).collect();

    let header = wav::Header::new(WAV_FORMAT_IEEE_FLOAT, 1, 44100, 32);
    let mut out = std::fs::File::create("out.wav").unwrap();

    wav::write(header, &wav::BitDepth::ThirtyTwoFloat(data), &mut out).unwrap();
}
