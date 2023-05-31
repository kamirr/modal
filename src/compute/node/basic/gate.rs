use std::sync::Arc;

use serde::{Deserialize, Serialize};

use crate::compute::{
    node::{inputs::gate::GateInput, Input, Node, NodeEvent},
    Value,
};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Gate {
    gate: Arc<GateInput>,
    out: f32,
}

impl Gate {
    pub fn new() -> Self {
        Gate {
            gate: Arc::new(GateInput::new(0.5)),
            out: 0.0,
        }
    }
}

#[typetag::serde]
impl Node for Gate {
    fn feed(&mut self, data: &[Value]) -> Vec<NodeEvent> {
        self.out = if self.gate.gate(&data[0]) { 1.0 } else { 0.0 };

        Default::default()
    }

    fn read(&self, out: &mut [Value]) {
        out[0] = Value::Float(self.out)
    }

    fn inputs(&self) -> Vec<Input> {
        vec![Input::stateful("gate", &self.gate)]
    }
}

pub fn gate() -> Box<dyn Node> {
    Box::new(Gate::new())
}
