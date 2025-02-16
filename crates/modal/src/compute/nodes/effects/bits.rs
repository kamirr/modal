use std::sync::Arc;

use serde::{Deserialize, Serialize};

use runtime::{
    node::{Input, Node, NodeEvent},
    Value, ValueKind,
};

use crate::compute::inputs::slider::SliderInput;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Bits {
    bits: Arc<SliderInput>,
    out: f32,
}

#[typetag::serde]
impl Node for Bits {
    fn feed(&mut self, data: &[Value]) -> Vec<NodeEvent> {
        let sig = data[0].as_float().unwrap_or(0.0);

        let bits = self.bits.as_f32(&data[1]);
        let states = (2f32).powf(bits - 1.0);

        let quantized = (sig.clamp(-1.0, 1.0) * states) as i16;
        self.out = quantized as f32 / states;

        Default::default()
    }

    fn read(&self, out: &mut [Value]) {
        out[0] = Value::Float(self.out)
    }

    fn inputs(&self) -> Vec<Input> {
        vec![
            Input::new("sig", ValueKind::Float),
            Input::stateful("bits", &self.bits),
        ]
    }
}

pub fn bits() -> Box<dyn Node> {
    Box::new(Bits {
        bits: Arc::new(SliderInput::new(1.0, 1.0, 16.0)),
        out: 0.0,
    })
}
