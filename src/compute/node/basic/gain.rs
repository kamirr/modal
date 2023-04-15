use std::sync::Arc;

use serde::Serialize;

use crate::compute::node::{inputs::positive::PositiveInput, Input, InputUi, Node, NodeEvent};

#[derive(Clone, Debug, Serialize)]
struct Gain {
    s1: Arc<PositiveInput>,
    out: f32,
}

impl Node for Gain {
    fn feed(&mut self, data: &[Option<f32>]) -> Vec<NodeEvent> {
        let s0 = data[0].unwrap_or(0.0);
        let s1 = data[1].unwrap_or(self.s1.value());

        self.out = s0 * s1;

        Default::default()
    }

    fn read(&self) -> f32 {
        self.out
    }

    fn inputs(&self) -> Vec<Input> {
        vec![Input::new("sig 0"), Input::with_default("sig 1", &self.s1)]
    }
}

pub fn gain() -> Box<dyn Node> {
    Box::new(Gain {
        s1: Arc::new(PositiveInput::new(1.0)),
        out: 0.0,
    })
}
