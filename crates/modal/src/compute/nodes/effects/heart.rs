use std::sync::Arc;

use serde::{Deserialize, Serialize};

use runtime::{
    node::{Input, Node, NodeEvent},
    Value, ValueKind,
};

use crate::compute::inputs::real::RealInput;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Heart {
    osc: Arc<RealInput>,
    density: Arc<RealInput>,
    out: f32,
}

#[typetag::serde]
impl Node for Heart {
    fn feed(&mut self, data: &[Value]) -> Vec<NodeEvent> {
        let osc = self.osc.get_f32(&data[0]).clamp(-1.0, 1.0) * 2.0;
        let density = self.density.get_f32(&data[1]);
        let mix = data[2].as_float().unwrap_or_default();
        self.out = osc.powi(2).powf(1.0 / 3.0)
            + (std::f32::consts::E / 3.0)
                * (4.0 - osc * osc).sqrt()
                * (mix + (density * std::f32::consts::PI * osc).sin());

        Default::default()
    }

    fn read(&self, out: &mut [Value]) {
        out[0] = Value::Float(self.out)
    }

    fn inputs(&self) -> Vec<Input> {
        vec![
            Input::stateful("oscillator", &self.osc),
            Input::stateful("density", &self.density),
            Input::new("mix", ValueKind::Float),
        ]
    }
}

pub fn heart() -> Box<dyn Node> {
    Box::new(Heart {
        osc: Arc::new(RealInput::new(0.0)),
        density: Arc::new(RealInput::new(17.0)),
        out: 0.0,
    })
}
