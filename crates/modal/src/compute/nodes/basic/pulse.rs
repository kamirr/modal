use std::sync::Arc;

use serde::{Deserialize, Serialize};

use crate::compute::inputs::{
    real::RealInput,
    time::TimeInput,
    trigger::{TriggerInput, TriggerMode},
};
use runtime::{
    node::{Input, Node, NodeEvent},
    Value,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
enum PulseState {
    Idle,
    Up(f32),
}

impl PulseState {
    fn step(&mut self) -> f32 {
        match self {
            PulseState::Idle => 0.0,
            &mut PulseState::Up(t) => {
                if t >= 1.0 {
                    *self = PulseState::Up(t - 1.0);
                    1.0
                } else {
                    *self = PulseState::Idle;
                    t.min(0.0)
                }
            }
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Pulse {
    trigger: Arc<TriggerInput>,
    time: Arc<TimeInput>,
    value: Arc<RealInput>,
    state: PulseState,
    out: f32,
}

impl Pulse {
    fn new(trigger_level: f32) -> Self {
        Pulse {
            trigger: Arc::new(TriggerInput::new(TriggerMode::Up, trigger_level)),
            time: Arc::new(TimeInput::new(4410.0)),
            value: Arc::new(RealInput::new(1.0)),
            state: PulseState::Idle,
            out: 0.0,
        }
    }
}

#[typetag::serde]
impl Node for Pulse {
    fn feed(&mut self, data: &[Value]) -> Vec<NodeEvent> {
        if self.trigger.trigger(&data[0]) {
            self.state = PulseState::Up(self.time.get_samples(&data[1]));
        }

        let gain = self.state.step();
        self.out = gain * self.value.get_f32(&data[2]);

        Default::default()
    }

    fn read(&self, out: &mut [Value]) {
        out[0] = Value::Float(self.out)
    }

    fn inputs(&self) -> Vec<Input> {
        vec![
            Input::stateful("trigger", &self.trigger),
            Input::stateful("length", &self.time),
            Input::stateful("value", &self.value),
        ]
    }
}

pub fn pulse() -> Box<dyn Node> {
    Box::new(Pulse::new(0.5))
}
