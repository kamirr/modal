use std::sync::Arc;

use crate::compute::inputs::real::RealInput;
use runtime::{
    node::{Input, Node, NodeEvent},
    ExternInputs, Output, Value, ValueKind,
};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Difference {
    a: Arc<RealInput>,
    b: Arc<RealInput>,
    out: f32,
}

#[typetag::serde]
impl Node for Difference {
    fn feed(&mut self, _inputs: &ExternInputs, data: &[Value]) -> Vec<NodeEvent> {
        self.out = self.a.get_f32(&data[0]) - self.b.get_f32(&data[1]);

        Default::default()
    }

    fn read(&self, out: &mut [Value]) {
        out[0] = Value::Float(self.out)
    }

    fn inputs(&self) -> Vec<Input> {
        vec![Input::stateful("a", &self.a), Input::stateful("b", &self.b)]
    }

    fn output(&self) -> Vec<Output> {
        vec![Output::new("a-b", ValueKind::Float)]
    }
}

pub fn difference() -> Box<dyn Node> {
    Box::new(Difference {
        a: Arc::new(RealInput::new(0.0)),
        b: Arc::new(RealInput::new(0.0)),
        out: 0.0,
    })
}
