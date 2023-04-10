use crate::param;

use super::{Node, NodeList, NodeMeta, Param, ParamValue};
use std::{collections::VecDeque, f32::consts::PI};

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

    fn set_param(&mut self, value: &[Param]) {
        self.coeffs = value[0].0[0].as_fdyn().into();
        while self.data.len() < self.coeffs.len() {
            self.data.push_back(0.0);
        }
        while self.data.len() > self.coeffs.len() {
            self.data.pop_back();
        }
    }

    fn get_param(&self) -> Vec<Param> {
        vec![Param(vec![ParamValue::FDyn(self.coeffs.clone())])]
    }

    fn meta(&self) -> NodeMeta {
        NodeMeta::new(["sig"], [("coeffs", param!(_ FDyn))])
    }
}

pub fn fir() -> Fir {
    Fir::new([0.5, 0.5])
}

#[derive(Clone, Debug)]
struct Biquad {
    a: [f32; 3],
    b: [f32; 3],
    in_hist: [f32; 3],
    out_hist: [f32; 2],
}

impl Biquad {
    fn new(a: [f32; 3], b: [f32; 3]) -> Self {
        Biquad {
            a,
            b,
            in_hist: [0.0; 3],
            out_hist: [0.0; 2],
        }
    }

    fn set_factors(&mut self, a: [f32; 3], b: [f32; 3]) {
        self.a = a;
        self.b = b;
    }

    fn next(&mut self, input: f32) {
        self.in_hist = [self.in_hist[1], self.in_hist[2], input];
        let out = (self.b[0] / self.a[0]) * self.in_hist[2]
            + (self.b[1] / self.a[0]) * self.in_hist[1]
            + (self.b[2] / self.a[0]) * self.in_hist[0]
            - (self.a[1] / self.a[0]) * self.out_hist[1]
            - (self.a[2] / self.a[0]) * self.out_hist[0];
        //dbg!(out);

        self.out_hist = [self.out_hist[1], out];
    }
}

#[derive(Clone, Debug)]
pub struct Lpf {
    inner: Biquad,
    q: f32,
    f0: f32,
}

impl Lpf {
    fn new(q: f32, f0: f32) -> Self {
        let (a, b) = Self::factors(q, f0);
        Lpf {
            inner: Biquad::new(a, b),
            q,
            f0,
        }
    }

    fn factors(q: f32, f0: f32) -> ([f32; 3], [f32; 3]) {
        let w0 = 2.0 * PI * f0 / 44100.0;
        let alpha = w0.sin() / 2.0 / q;

        (
            [1.0 + alpha, -2.0 * w0.cos(), 1.0 - alpha],
            [
                (1.0 - w0.cos()) / 2.0,
                1.0 - w0.cos(),
                (1.0 - w0.cos()) / 2.0,
            ],
        )
    }
}

impl Node for Lpf {
    fn feed(&mut self, samples: &[f32]) {
        self.inner.next(samples[0]);
    }

    fn read(&self) -> f32 {
        self.inner.out_hist[1]
    }

    fn get_param(&self) -> Vec<Param> {
        vec![
            Param(vec![ParamValue::F(self.q)]),
            Param(vec![ParamValue::F(self.f0)]),
        ]
    }

    fn set_param(&mut self, value: &[Param]) {
        self.q = value[0].0[0].as_f();
        self.f0 = value[1].0[0].as_f();
        let (a, b) = Self::factors(self.q, self.f0);
        self.inner.set_factors(a, b);
    }

    fn meta(&self) -> NodeMeta {
        NodeMeta::new(["sig"], [("q", param!(_ F)), ("f0", param!(_ F))])
    }
}

fn lpf() -> Lpf {
    Lpf::new(5.0, 440.0)
}

pub struct Filters;

impl NodeList for Filters {
    fn all(&self) -> Vec<(fn() -> Box<dyn Node>, &'static str)> {
        vec![
            (|| Box::new(fir()), "FIR Filter"),
            (|| Box::new(lpf()), "LPF Filter"),
        ]
    }
}
