use std::sync::Arc;

use crate::compute::{
    node::{inputs::real::RealInput, Input, Node, NodeConfig, NodeEvent},
    Value, ValueKind,
};
use serde::{Deserialize, Serialize};

use super::curve::CurveConfig;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transform {
    config: Arc<CurveConfig>,

    in_min: Arc<RealInput>,
    in_max: Arc<RealInput>,
    out_min: Arc<RealInput>,
    out_max: Arc<RealInput>,

    out: f32,
}

impl Transform {
    pub fn new() -> Self {
        Transform {
            config: Arc::new(CurveConfig::new()),
            in_min: Arc::new(RealInput::new(-1.0)),
            in_max: Arc::new(RealInput::new(1.0)),
            out_min: Arc::new(RealInput::new(-1.0)),
            out_max: Arc::new(RealInput::new(1.0)),
            out: 0.0,
        }
    }
}

#[typetag::serde]
impl Node for Transform {
    fn feed(&mut self, data: &[Value]) -> Vec<NodeEvent> {
        let signal = data[0].as_float().unwrap_or(0.0);
        let in_min = self.in_min.get_f32(&data[1]);
        let in_max = self.in_max.get_f32(&data[2]);
        let out_min = self.out_min.get_f32(&data[3]);
        let out_max = self.out_max.get_f32(&data[4]);

        let idx_0_1 = (signal - in_min) / (in_max - in_min);

        let raw_out = {
            let values = self.config.values();

            let idx_f32 = idx_0_1 * values.len() as f32;
            let idx = idx_f32 as usize;
            let idx = idx.clamp(0, values.len() - 2);

            let curr = values[idx];
            let next = values[idx + 1];
            let f = idx_f32 - idx as f32;

            curr * (1.0 - f) + next * f
        };

        self.out = raw_out / 100.0 * (out_max - out_min) + out_min;

        Default::default()
    }

    fn read(&self, out: &mut [Value]) {
        out[0] = Value::Float(self.out)
    }

    fn config(&self) -> Option<Arc<dyn NodeConfig>> {
        Some(Arc::clone(&self.config) as Arc<_>)
    }

    fn inputs(&self) -> Vec<Input> {
        vec![
            Input::new("sig", ValueKind::Float),
            Input::stateful("in min", &self.in_min),
            Input::stateful("in max", &self.in_max),
            Input::stateful("out min", &self.out_min),
            Input::stateful("out max", &self.out_max),
        ]
    }
}

pub fn transform() -> Box<dyn Node> {
    Box::new(Transform::new())
}
