use std::sync::Arc;

use serde::{Deserialize, Serialize};

use crate::compute::node::{
    inputs::{
        time::TimeInput,
        trigger::{TriggerInput, TriggerMode},
    },
    Input, InputUi, Node, NodeEvent,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
enum PulseState {
    Idle,
    Up(usize),
}

impl PulseState {
    fn step(&mut self) -> f32 {
        match self {
            PulseState::Idle => 0.0,
            &mut PulseState::Up(n) => {
                if n == 0 {
                    *self = PulseState::Idle
                } else {
                    *self = PulseState::Up(n - 1)
                }

                1.0
            }
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Pulse {
    trigger: Arc<TriggerInput>,
    time: Arc<TimeInput>,
    state: PulseState,
    out: f32,
}

impl Pulse {
    fn new(trigger_level: f32) -> Self {
        Pulse {
            trigger: Arc::new(TriggerInput::new(TriggerMode::Up, trigger_level)),
            time: Arc::new(TimeInput::new(4410)),
            state: PulseState::Idle,
            out: 0.0,
        }
    }
}

#[typetag::serde]
impl Node for Pulse {
    fn feed(&mut self, data: &[Option<f32>]) -> Vec<NodeEvent> {
        if self.trigger.value(data[0]) > 0.5 {
            self.state = PulseState::Up(self.time.value(data[1]) as usize);
        }

        self.out = self.state.step();

        Default::default()
    }

    fn read(&self) -> f32 {
        self.out
    }

    fn inputs(&self) -> Vec<Input> {
        vec![
            Input::with_default("trigger", &self.trigger),
            Input::with_default("length", &self.time),
        ]
    }
}

pub fn pulse() -> Box<dyn Node> {
    Box::new(Pulse::new(0.5))
}
