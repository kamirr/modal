use std::sync::Arc;

use serde::{Deserialize, Serialize};

use runtime::{
    node::{Input, Node, NodeEvent},
    Value, ValueKind,
};

use crate::compute::inputs::trigger::{TriggerInput, TriggerMode};

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
    fn feed(&mut self, data: &[Value]) -> Vec<NodeEvent> {
        if self.trigger.trigger(&data[0]) {
            self.out = data[1].as_float().unwrap_or_default();
        }

        Default::default()
    }

    fn read(&self, out: &mut [Value]) {
        out[0] = Value::Float(self.out)
    }

    fn inputs(&self) -> Vec<Input> {
        vec![
            Input::stateful("trigger", &self.trigger),
            Input::new("signal", ValueKind::Float),
        ]
    }
}

pub fn latch() -> Box<dyn Node> {
    Box::new(Latch::new())
}
