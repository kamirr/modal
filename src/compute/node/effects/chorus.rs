use std::{collections::VecDeque, sync::Arc};

use serde::{Deserialize, Serialize};

use crate::compute::{
    node::{
        inputs::{percentage::PercentageInput, time::TimeInput},
        Input, Node, NodeEvent,
    },
    Value, ValueKind,
};

#[derive(Clone, Debug, Serialize, Deserialize)]
struct Chorus {
    delay: VecDeque<f32>,
    delay_in: Arc<TimeInput>,
    width_in: Arc<PercentageInput>,
    mix_in: Arc<PercentageInput>,
    out: f32,
}

impl Chorus {
    pub fn tap_at(&self, ms: f32) -> f32 {
        let fuzzy_idx = ms / 1000.0 * 44100.0;
        let idx_low = fuzzy_idx.floor() as usize;
        let idx_high = idx_low + 1;

        let high_weight = fuzzy_idx - idx_low as f32;
        let low_weight = 1.0 - high_weight;

        self.delay[idx_low] * low_weight + self.delay[idx_high] * high_weight
    }
}

#[typetag::serde]
impl Node for Chorus {
    fn feed(&mut self, data: &[Value]) -> Vec<NodeEvent> {
        let sample = data[0].as_float().unwrap_or(0.0);
        self.delay.push_front(sample);
        self.delay.pop_back();

        let delay_out = *self.delay.front().unwrap();

        let osc_val = data[1].as_float().unwrap_or(0.0).clamp(-1.0, 1.0);
        let delay = self.delay_in.get_ms(&data[2]).clamp(0.0, 50.0);
        let width = self.width_in.get_f32(&data[3]);
        let tap_t = (delay + osc_val * delay * width).clamp(0.0, 50.0);
        let tap_out = self.tap_at(tap_t);

        let mix = self.mix_in.get_f32(&data[4]);

        self.out = delay_out * (1.0 - mix) + tap_out * mix;

        Default::default()
    }

    fn read(&self, out: &mut [Value]) {
        out[0] = Value::Float(self.out)
    }

    fn inputs(&self) -> Vec<Input> {
        vec![
            Input::new("sig", ValueKind::Float),
            Input::new("osc", ValueKind::Float),
            Input::stateful("delay", &self.delay_in),
            Input::stateful("width", &self.width_in),
            Input::stateful("mix", &self.mix_in),
        ]
    }
}

pub fn chorus() -> Box<dyn Node> {
    Box::new(Chorus {
        delay: std::iter::repeat(0.0).take(2205 + 10).collect(),
        delay_in: Arc::new(TimeInput::new(882.0)),
        width_in: Arc::new(PercentageInput::new(10.0)),
        mix_in: Arc::new(PercentageInput::new(50.0)),
        out: 0.0,
    })
}
