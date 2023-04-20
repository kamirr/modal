use std::{collections::VecDeque, sync::Arc};

use serde::{Deserialize, Serialize};

use crate::compute::node::{inputs::time::TimeInput, Input, InputUi, Node, NodeEvent};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Delay {
    time_in: Arc<TimeInput>,
    data: VecDeque<f32>,
    out: f32,
}

impl Delay {
    fn new(len: usize) -> Self {
        Delay {
            time_in: Arc::new(TimeInput::new(len)),
            data: std::iter::repeat(0.0).take(len).collect(),
            out: 0.0,
        }
    }
}

#[typetag::serde]
impl Node for Delay {
    fn feed(&mut self, data: &[Option<f32>]) -> Vec<NodeEvent> {
        let target_len = self.time_in.value(data[1]) as usize;
        while target_len > self.data.len() {
            self.data.push_back(0.0);
        }
        if target_len < self.data.len() {
            self.data.drain(0..(self.data.len() - target_len));
        }

        self.data.push_back(data[0].unwrap_or(0.0));
        self.out = self.data.pop_front().unwrap();

        Default::default()
    }

    fn read(&self) -> f32 {
        self.out
    }

    fn inputs(&self) -> Vec<Input> {
        vec![
            Input::new("sig"),
            Input::with_default("time", &self.time_in),
        ]
    }
}

pub fn delay() -> Box<dyn Node> {
    Box::new(Delay::new(4410))
}
