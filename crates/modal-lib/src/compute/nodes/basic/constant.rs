use std::sync::Arc;

use serde::{Deserialize, Serialize};

use runtime::{
    node::{Input, Node, NodeEvent},
    ExternInputs, Value,
};

use crate::compute::inputs::real::RealInput;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Constant {
    value: Arc<RealInput>,
    out: f32,
}

#[typetag::serde]
impl Node for Constant {
    fn feed(&mut self, _inputs: &ExternInputs, data: &[Value]) -> Vec<NodeEvent> {
        self.out = self.value.get_f32(&data[0]);

        Default::default()
    }

    fn read(&self, out: &mut [Value]) {
        out[0] = Value::Float(self.out)
    }

    fn inputs(&self) -> Vec<Input> {
        vec![Input::stateful("value", &self.value)]
    }
}

pub fn constant() -> Box<dyn Node> {
    Box::new(Constant {
        value: Arc::new(RealInput::new(0.0)),
        out: 0.0,
    })
}
