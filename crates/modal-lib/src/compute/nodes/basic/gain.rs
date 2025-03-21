use std::sync::Arc;

use serde::{Deserialize, Serialize};

use crate::compute::inputs::gain::GainInput;
use runtime::{
    node::{Input, Node, NodeEvent},
    ExternInputs, Value, ValueKind,
};

#[derive(Clone, Debug, Serialize, Deserialize)]
struct Gain {
    s1: Arc<GainInput>,
    out: f32,
}

#[typetag::serde]
impl Node for Gain {
    fn feed(&mut self, _inputs: &ExternInputs, data: &[Value]) -> Vec<NodeEvent> {
        let s0 = data[0].as_float().unwrap_or(0.0);
        let s1 = self.s1.get_multiplier(&data[1]);

        self.out = s0 * s1;

        Default::default()
    }

    fn read(&self, out: &mut [Value]) {
        out[0] = Value::Float(self.out)
    }

    fn inputs(&self) -> Vec<Input> {
        vec![
            Input::new("sig 0", ValueKind::Float),
            Input::stateful("sig 1", &self.s1),
        ]
    }
}

pub fn gain() -> Box<dyn Node> {
    Box::new(Gain {
        s1: Arc::new(GainInput::unit()),
        out: 0.0,
    })
}
