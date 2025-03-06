use std::sync::Arc;

use serde::{Deserialize, Serialize};

use runtime::{
    node::{Input, Node, NodeEvent},
    ExternInputs, Value,
};

use crate::compute::inputs::gate::{GateInput, GateInputState};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Gate {
    gate: Arc<GateInput>,
    gate_state: GateInputState,
    out: f32,
}

impl Default for Gate {
    fn default() -> Self {
        Self::new()
    }
}

impl Gate {
    pub fn new() -> Self {
        Gate {
            gate: Arc::new(GateInput::new(0.5)),
            gate_state: GateInputState::default(),
            out: 0.0,
        }
    }
}

#[typetag::serde]
impl Node for Gate {
    fn feed(&mut self, _inputs: &ExternInputs, data: &[Value]) -> Vec<NodeEvent> {
        self.out = if self.gate.gate(&mut self.gate_state, &data[0]) {
            1.0
        } else {
            0.0
        };

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
