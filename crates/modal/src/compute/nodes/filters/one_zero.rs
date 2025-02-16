use serde::{Deserialize, Serialize};

use crate::{
    compute::inputs::slider::SliderInput,
    node::{Input, Node, NodeEvent},
};
use runtime::{ExternInputs, Value, ValueKind};

use std::sync::Arc;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct OneZero {
    zero: Arc<SliderInput>,
    in_hist: [f32; 2],
    out: f32,
}

impl OneZero {
    pub fn new(zero: f32) -> Self {
        OneZero {
            zero: Arc::new(SliderInput::new(zero, -1.0, 1.0)),
            in_hist: [1.0, 0.0],
            out: 0.0,
        }
    }

    pub fn next(&mut self, input: f32, param: &Value) {
        let b = self.coeffs(param);

        self.in_hist[0] = input;
        self.out = b[1] * self.in_hist[1] + b[0] * self.in_hist[0];
        self.in_hist[1] = self.in_hist[0];
    }

    fn coeffs(&self, param: &Value) -> [f32; 2] {
        let zero = self.zero.as_f32(param);

        let b0 = if zero > 0.0 {
            1.0 / (1.0 + zero)
        } else {
            1.0 / (1.0 - zero)
        };
        let b1 = -zero * b0;

        [b0, b1]
    }
}

#[typetag::serde]
impl Node for OneZero {
    fn feed(&mut self, _inputs: &ExternInputs, data: &[Value]) -> Vec<NodeEvent> {
        self.next(data[0].as_float().unwrap_or_default(), &data[1]);

        Default::default()
    }

    fn read(&self, out: &mut [Value]) {
        out[0] = Value::Float(self.out)
    }

    fn inputs(&self) -> Vec<Input> {
        vec![
            Input::new("sig", ValueKind::Float),
            Input::stateful("zero", &self.zero),
        ]
    }
}

pub fn one_zero() -> Box<dyn Node> {
    Box::new(OneZero::new(-1.0))
}
