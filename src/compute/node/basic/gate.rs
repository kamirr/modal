use std::sync::Arc;

use serde::{Deserialize, Serialize};

use crate::compute::node::{inputs::gate::GateInput, Input, InputUi, Node, NodeEvent};

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
    fn feed(&mut self, data: &[Option<f32>]) -> Vec<NodeEvent> {
        self.out = self.gate.value(data[0]);

        Default::default()
    }

    fn read(&self) -> f32 {
        self.out
    }

    fn inputs(&self) -> Vec<Input> {
        vec![Input::with_default("gate", &self.gate)]
    }
}

pub fn gate() -> Box<dyn Node> {
    Box::new(Gate::new())
}
