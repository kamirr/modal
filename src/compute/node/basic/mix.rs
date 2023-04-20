use std::sync::Arc;

use serde::{Deserialize, Serialize};

use crate::compute::node::{inputs::slider::SliderInput, Input, InputUi, Node, NodeEvent};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Mix {
    ratio: Arc<SliderInput>,
    out: f32,
}

impl Mix {
    pub fn new() -> Self {
        Mix {
            ratio: Arc::new(SliderInput::new(0.5, 0.0, 1.0)),
            out: 0.0,
        }
    }
}

#[typetag::serde]
impl Node for Mix {
    fn feed(&mut self, data: &[Option<f32>]) -> Vec<NodeEvent> {
        let sig0 = data[0].unwrap_or(0.0);
        let sig1 = data[1].unwrap_or(0.0);
        let ratio = self.ratio.value(data[2]);

        self.out = sig0 * ratio + sig1 * (1.0 - ratio);

        Default::default()
    }

    fn read(&self) -> f32 {
        self.out
    }

    fn inputs(&self) -> Vec<Input> {
        vec![
            Input::new("sig 0"),
            Input::new("sig 1"),
            Input::with_default("mix", &self.ratio),
        ]
    }
}

pub fn mix() -> Box<dyn Node> {
    Box::new(Mix::new())
}
