use std::{collections::VecDeque, sync::Arc};

use serde::{Deserialize, Serialize};

use runtime::{
    node::{Input, Node, NodeEvent},
    ExternInputs, Value, ValueKind,
};

use crate::compute::inputs::time::TimeInput;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ReverseDelay {
    time_in: Arc<TimeInput>,
    len: usize,
    playback: VecDeque<f32>,
    record: VecDeque<f32>,
    out: f32,
}

impl ReverseDelay {
    pub fn new() -> Self {
        let len = 44100 / 2;
        ReverseDelay {
            time_in: Arc::new(TimeInput::new(len as f32)),
            len,
            playback: VecDeque::new(),
            record: VecDeque::new(),
            out: 0.0,
        }
    }
}

#[typetag::serde]
impl Node for ReverseDelay {
    fn feed(&mut self, _inputs: &ExternInputs, data: &[Value]) -> Vec<NodeEvent> {
        let sample = data[0].as_float().unwrap_or_default();
        let len = self.time_in.get_samples(&data[1]).max(1.0) as usize;

        self.record.push_back(sample);
        if self.record.len() >= len {
            self.playback.clone_from(&self.record);
            self.record.clear();
        }

        self.out = self.playback.pop_back().unwrap_or_default();

        Default::default()
    }

    fn read(&self, out: &mut [Value]) {
        out[0] = Value::Float(self.out)
    }

    fn inputs(&self) -> Vec<Input> {
        vec![
            Input::new("sig", ValueKind::Float),
            Input::stateful("time", &self.time_in),
        ]
    }
}

pub fn reverse_delay() -> Box<dyn Node> {
    Box::new(ReverseDelay::new())
}
