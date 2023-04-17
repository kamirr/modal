use std::sync::Arc;

use serde::{Deserialize, Serialize};

use crate::compute::node::{inputs::real::RealInput, Input, InputUi, Node, NodeEvent};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Constant {
    value: Arc<RealInput>,
    out: f32,
}

#[typetag::serde]
impl Node for Constant {
    fn feed(&mut self, data: &[Option<f32>]) -> Vec<NodeEvent> {
        self.out = data[0].unwrap_or(self.value.value());

        Default::default()
    }

    fn read(&self) -> f32 {
        self.out
    }

    fn inputs(&self) -> Vec<Input> {
        vec![Input {
            name: "value".into(),
            default_value: Some(Arc::clone(&self.value) as Arc<_>),
        }]
    }
}

pub fn constant() -> Box<dyn Node> {
    Box::new(Constant {
        value: Arc::new(RealInput::new(0.0)),
        out: 0.0,
    })
}
