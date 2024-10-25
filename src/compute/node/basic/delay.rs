use std::{collections::VecDeque, sync::Arc};

use serde::{Deserialize, Serialize};

use crate::compute::{
    node::{
        inputs::{percentage::PercentageInput, time::TimeInput},
        Input, Node, NodeEvent,
    },
    Value, ValueKind,
};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Delay {
    time_in: Arc<TimeInput>,
    feedback: Arc<PercentageInput>,
    data: VecDeque<f32>,
    out: f32,
}

impl Delay {
    pub fn new(len: usize) -> Self {
        Delay {
            time_in: Arc::new(TimeInput::new(len)),
            feedback: Arc::new(PercentageInput::new(0.0)),
            data: std::iter::repeat(0.0).take(len).collect(),
            out: 0.0,
        }
    }

    pub fn set_len(&self, samples: usize) {
        self.time_in.set_samples(samples);
    }

    pub fn len(&self) -> usize {
        self.data.len()
    }

    pub fn clear(&mut self) {
        for sample in &mut self.data {
            *sample = 0.0;
        }
    }
}

#[typetag::serde]
impl Node for Delay {
    fn feed(&mut self, data: &[Value]) -> Vec<NodeEvent> {
        let target_len = self.time_in.get_samples(&data[1]);
        let feedback = self.feedback.get_f32(&data[2]);

        while target_len > self.data.len() {
            self.data.push_back(0.0);
        }
        if target_len < self.data.len() {
            self.data.drain(0..(self.data.len() - target_len));
        }

        self.data
            .push_back(data[0].as_float().unwrap_or(0.0) + self.data.front().unwrap() * feedback);
        self.out = self.data.pop_front().unwrap();

        Default::default()
    }

    fn read(&self, out: &mut [Value]) {
        out[0] = Value::Float(self.out)
    }

    fn inputs(&self) -> Vec<Input> {
        vec![
            Input::new("sig", ValueKind::Float),
            Input::stateful("time", &self.time_in),
            Input::stateful("feedback", &self.feedback),
        ]
    }
}

pub fn delay() -> Box<dyn Node> {
    Box::new(Delay::new(4410))
}
