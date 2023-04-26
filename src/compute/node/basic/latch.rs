use std::sync::Arc;

use serde::{Deserialize, Serialize};

use crate::compute::node::{
    inputs::trigger::{TriggerInput, TriggerMode},
    Input, InputUi, Node, NodeEvent,
};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Latch {
    trigger: Arc<TriggerInput>,
    out: f32,
}

impl Latch {
    pub fn new() -> Self {
        Latch {
            trigger: Arc::new(TriggerInput::new(TriggerMode::Up, 0.5)),
            out: 0.0,
        }
    }
}

#[typetag::serde]
impl Node for Latch {
    fn feed(&mut self, data: &[Option<f32>]) -> Vec<NodeEvent> {
        let trigger: f32 = self.trigger.value(data[0]);
        let sig = data[1].unwrap_or(0.0);

        if trigger > 0.5 {
            self.out = sig;
        }

        Default::default()
    }

    fn read(&self) -> f32 {
        self.out
    }

    fn inputs(&self) -> Vec<Input> {
        vec![
            Input::with_default("trigger", &self.trigger),
            Input::new("signal"),
        ]
    }
}

pub fn latch() -> Box<dyn Node> {
    Box::new(Latch::new())
}
