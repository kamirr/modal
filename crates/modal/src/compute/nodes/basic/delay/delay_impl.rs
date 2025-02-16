use std::collections::VecDeque;

use rubato::{FftFixedInOut, Resampler};
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
pub enum ResizeStrategy {
    Resample {
        /// Determines resampling frequency
        ///
        /// Resampling won't be performed more than once per freq_div samples.
        freq_div: u32,
    },
    ZeroFillDrain,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RawDelay {
    interpolation: Interpolation,
    resize_strat: ResizeStrategy,
    data: VecDeque<f32>,
    last_out: f32,
    resample_target: Option<(u32, f32)>,
    resample_buf: Vec<f32>,
}

impl RawDelay {
    pub fn new(len: usize) -> Self {
        RawDelay {
            interpolation: Interpolation::None,
            resize_strat: ResizeStrategy::ZeroFillDrain,
            data: std::iter::repeat(0.0).take(len).collect(),
            last_out: 0.0,
            resample_target: None,
            resample_buf: Vec::new(),
        }
    }

    pub fn new_linear(len: f32) -> Self {
        let len_i = len.round() as usize;
        RawDelay {
            interpolation: Interpolation::linear(len),
            resize_strat: ResizeStrategy::ZeroFillDrain,
            data: std::iter::repeat(0.0).take(len_i).collect(),
            last_out: 0.0,
            resample_target: None,
            resample_buf: Vec::new(),
        }
    }

    pub fn new_allpass(len: f32) -> Self {
        let len_i = len.round() as usize;
        RawDelay {
            interpolation: Interpolation::allpass(len),
            resize_strat: ResizeStrategy::ZeroFillDrain,
            data: std::iter::repeat(0.0).take(len_i).collect(),
            last_out: 0.0,
            resample_target: None,
            resample_buf: Vec::new(),
        }
    }

    pub fn resize_strategy(&mut self, strat: ResizeStrategy) {
        self.resize_strat = strat;
    }

    pub fn len(&self) -> f32 {
        self.data.len() as f32
    }

    fn resize_via_resample(&mut self, new_len: f32) {
        self.interpolation = match self.interpolation {
            Interpolation::Alpass { .. } => Interpolation::allpass(new_len),
            Interpolation::Linear { .. } => Interpolation::linear(new_len),
            Interpolation::None => Interpolation::None,
        };

        let len_i = new_len.round() as usize;

        let mut processor =
            FftFixedInOut::<f32>::new(self.data.len(), len_i, self.data.len() * 2, 1).unwrap();

        self.data.extend(std::iter::repeat_n(0.0, self.data.len()));
        let in_buf = self.data.make_contiguous();

        self.resample_buf.resize(len_i * 2, 0.0);
        let out_buf = self.resample_buf.as_mut_slice();

        processor
            .process_into_buffer(&[in_buf], &mut [&mut out_buf[..]], None)
            .unwrap();

        let out = &out_buf[len_i..];
        assert_eq!(out.len(), len_i);

        self.data.clear();
        self.data.extend(out.iter().copied());
    }

    fn resize_via_extend(&mut self, new_len: f32) {
        self.interpolation = match self.interpolation {
            Interpolation::Alpass { .. } => Interpolation::allpass(new_len),
            Interpolation::Linear { .. } => Interpolation::linear(new_len),
            Interpolation::None => Interpolation::None,
        };

        let len_i = new_len.round() as usize;

        while len_i > self.data.len() {
            self.data.push_back(0.0);
        }

        if len_i < self.data.len() {
            self.data.drain(0..(self.data.len() - len_i));
        }
    }

    pub fn resize(&mut self, len: f32) {
        match self.resize_strat {
            ResizeStrategy::Resample { freq_div } => {
                if let Some((_cnt, target_len)) = &mut self.resample_target {
                    *target_len = len;
                } else {
                    self.resample_target = Some((freq_div, len));
                }
            }
            ResizeStrategy::ZeroFillDrain => {
                self.resize_via_extend(len);
            }
        }
    }

    pub fn push(&mut self, value: f32) {
        self.data.push_back(value);
        self.last_out = self.interpolation.compute(&mut self.data, self.last_out);

        let resample = if let Some((cnt, target_len)) = &mut self.resample_target {
            *cnt -= 1;
            if *cnt == 0 {
                Some(*target_len)
            } else {
                None
            }
        } else {
            None
        };

        if let Some(new_len) = resample {
            self.resample_target = None;
            self.resize_via_resample(new_len);
        }
    }

    pub fn last_out(&self) -> f32 {
        self.last_out
    }

    pub fn clear(&mut self) {
        self.data.iter_mut().for_each(|s| *s = 0.0);
        self.last_out = 0.0;
    }
}
