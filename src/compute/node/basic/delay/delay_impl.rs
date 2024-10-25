use std::collections::VecDeque;

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
enum Interpolation {
    Alpass { coeff: f32, ap_input: f32 },
    Linear { alpha: f32, om_alpha: f32 },
    None,
}

impl Interpolation {
    fn allpass(len: f32) -> Self {
        let len_i = len as usize;
        let alpha = len - len_i as f32;
        let coeff = (1.0 - alpha) / (1.0 + alpha);
        let ap_input = 0.0;

        Interpolation::Alpass { coeff, ap_input }
    }

    fn linear(len: f32) -> Self {
        let len_i = len as usize;
        let alpha = len - len_i as f32;
        let om_alpha = 1.0 - alpha;

        Interpolation::Linear { alpha, om_alpha }
    }

    fn compute(&mut self, data: &mut VecDeque<f32>, last_out: f32) -> f32 {
        match self {
            Interpolation::None => data.pop_front().unwrap(),
            Interpolation::Linear {
                ref alpha,
                ref om_alpha,
            } => {
                let d0 = data.pop_front().unwrap();
                let d1 = *data.front().unwrap();

                om_alpha * d0 + alpha * d1
            }
            Interpolation::Alpass {
                ref coeff,
                ap_input,
            } => {
                let d0 = data.pop_front().unwrap();
                let out = -coeff * last_out + *ap_input + (coeff * d0);
                *ap_input = d0;

                out
            }
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RawDelay {
    interpolation: Interpolation,
    data: VecDeque<f32>,
    last_out: f32,
}

impl RawDelay {
    pub fn new(len: usize) -> Self {
        RawDelay {
            interpolation: Interpolation::None,
            data: std::iter::repeat(0.0).take(len).collect(),
            last_out: 0.0,
        }
    }

    pub fn new_linear(len: f32) -> Self {
        let len_i = len.round() as usize;
        RawDelay {
            interpolation: Interpolation::linear(len),
            data: std::iter::repeat(0.0).take(len_i).collect(),
            last_out: 0.0,
        }
    }

    #[expect(dead_code)]
    pub fn new_allpass(len: f32) -> Self {
        let len_i = len.round() as usize;
        RawDelay {
            interpolation: Interpolation::allpass(len),
            data: std::iter::repeat(0.0).take(len_i).collect(),
            last_out: 0.0,
        }
    }

    pub fn len(&self) -> f32 {
        self.data.len() as f32
    }

    pub fn resize(&mut self, len: f32) {
        self.interpolation = match self.interpolation {
            Interpolation::Alpass { .. } => Interpolation::allpass(len),
            Interpolation::Linear { .. } => Interpolation::linear(len),
            Interpolation::None => Interpolation::None,
        };

        let new_len = len.round() as usize;
        while new_len > self.data.len() {
            self.data.push_back(0.0);
        }
        if new_len < self.data.len() {
            self.data.drain(0..(self.data.len() - new_len));
        }
    }

    pub fn push(&mut self, value: f32) {
        self.data.push_back(value);
        self.last_out = self.interpolation.compute(&mut self.data, self.last_out);
    }

    pub fn last_out(&self) -> f32 {
        self.last_out
    }

    pub fn clear(&mut self) {
        self.data.iter_mut().for_each(|s| *s = 0.0);
        self.last_out = 0.0;
    }
}
