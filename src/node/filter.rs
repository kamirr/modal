use super::Node;
use std::collections::VecDeque;

#[derive(Clone, Debug)]
pub struct Fir {
    coeffs: Vec<f32>,
    data: VecDeque<f32>,
    out: f32,
}

impl Fir {
    fn new(coeffs: impl Into<Vec<f32>>) -> Self {
        let mut coeffs: Vec<_> = coeffs.into();
        let sum: f32 = coeffs.iter().copied().map(f32::abs).sum();
        for c in &mut coeffs {
            *c /= sum;
        }
        let data = std::iter::repeat(0.0).take(coeffs.len()).collect();
        Fir {
            coeffs,
            data,
            out: 0.0,
        }
    }
}

impl Node for Fir {
    fn feed(&mut self, samples: &[f32]) {
        self.data.push_back(samples[0]);
        self.data.pop_front();

        self.out = 0.0;
        for (sample, coeff) in self.data.iter().zip(self.coeffs.iter()) {
            self.out += sample * coeff;
        }
    }

    fn read(&self) -> f32 {
        self.out
    }
}

pub fn fir(coeffs: impl Into<Vec<f32>>) -> Fir {
    Fir::new(coeffs)
}
