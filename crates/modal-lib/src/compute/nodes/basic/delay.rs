mod delay_impl;

use std::sync::Arc;

use crate::compute::inputs::{percentage::PercentageInput, time::TimeInput};
pub use delay_impl::{RawDelay, ResizeStrategy};
use runtime::{
    node::{Input, Node, NodeEvent},
    ExternInputs, Value, ValueKind,
};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Delay {
    time_in: Arc<TimeInput>,
    feedback: Arc<PercentageInput>,
    delay_impl: RawDelay,
}

impl Delay {
    pub fn new(delay_impl: RawDelay) -> Self {
        Delay {
            time_in: Arc::new(TimeInput::new(delay_impl.len())),
            feedback: Arc::new(PercentageInput::new(0.0)),
            delay_impl,
        }
    }
}

#[typetag::serde]
impl Node for Delay {
    fn feed(&mut self, _inputs: &ExternInputs, data: &[Value]) -> Vec<NodeEvent> {
        let target_len = self.time_in.get_samples(&data[1]);
        let feedback_gain = self.feedback.get_f32(&data[2]);

        let input = data[0].as_float().unwrap_or(0.0);
        let feedback = feedback_gain * self.delay_impl.last_out();

        let new_size = target_len;
        if self.delay_impl.len() != new_size {
            self.delay_impl.resize(new_size);
        }

        self.delay_impl.push(input + feedback);

        Default::default()
    }

    fn read(&self, out: &mut [Value]) {
        out[0] = Value::Float(self.delay_impl.last_out())
    }

    fn inputs(&self) -> Vec<Input> {
        vec![
            Input::new("sig", ValueKind::Float),
            Input::stateful("time", &self.time_in),
            Input::stateful("feedback", &self.feedback),
        ]
    }
}

pub fn delay(resize_strat: ResizeStrategy) -> Box<dyn Node> {
    Box::new(Delay::new({
        let mut delay = RawDelay::new(4410);
        delay.resize_strategy(resize_strat);
        delay
    }))
}
