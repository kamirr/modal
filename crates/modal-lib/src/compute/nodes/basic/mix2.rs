use std::sync::Arc;

use serde::{Deserialize, Serialize};

use runtime::{
    node::{Input, Node, NodeEvent},
    ExternInputs, Value, ValueKind,
};

use crate::compute::inputs::slider::SliderInput;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Mix2 {
    ratio: Arc<SliderInput>,
    out: f32,
}

impl Default for Mix2 {
    fn default() -> Self {
        Self::new()
    }
}

impl Mix2 {
    pub fn new() -> Self {
        Mix2 {
            ratio: Arc::new(SliderInput::new(0.5, 0.0, 1.0)),
            out: 0.0,
        }
    }
}

#[typetag::serde]
impl Node for Mix2 {
    fn feed(&mut self, _inputs: &ExternInputs, data: &[Value]) -> Vec<NodeEvent> {
        let sig0 = data[0].as_float().unwrap_or_default();
        let sig1 = data[1].as_float().unwrap_or_default();
        let ratio = self.ratio.as_f32(&data[2]);

        self.out = sig0 * ratio + sig1 * (1.0 - ratio);

        Default::default()
    }

    fn read(&self, out: &mut [Value]) {
        out[0] = Value::Float(self.out)
    }

    fn inputs(&self) -> Vec<Input> {
        vec![
            Input::new("sig 0", ValueKind::Float),
            Input::new("sig 1", ValueKind::Float),
            Input::stateful("mix2", &self.ratio),
        ]
    }
}

pub fn mix2() -> Box<dyn Node> {
    Box::new(Mix2::new())
}
