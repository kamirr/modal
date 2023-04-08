use std::{collections::VecDeque, fmt::Debug};

use super::Node;

#[derive(Clone, Debug)]
pub struct Placeholder;

impl Node for Placeholder {
    fn feed(&mut self, _samples: &[f32]) {
        unreachable!()
    }

    fn read(&self) -> f32 {
        unreachable!()
    }
}

#[derive(Clone, Debug)]
pub struct Add {
    out: f32,
}

impl Add {
    fn new() -> Self {
        Add { out: 0.0 }
    }
}

impl Node for Add {
    fn feed(&mut self, samples: &[f32]) {
        self.out = samples.iter().sum();
    }

    fn read(&self) -> f32 {
        self.out
    }
}

pub fn add() -> Add {
    Add::new()
}

#[derive(Clone, Debug)]
pub struct Delay {
    data: VecDeque<f32>,
    out: f32,
}

impl Delay {
    fn new(len: usize) -> Self {
        Delay {
            data: std::iter::repeat(0.0).take(len).collect(),
            out: 0.0,
        }
    }

    pub fn apply(&mut self, f: impl for<'a> Fn(&'a mut VecDeque<f32>)) {
        f(&mut self.data);
        self.out = self.data.pop_front().unwrap();
    }
}

impl Node for Delay {
    fn feed(&mut self, samples: &[f32]) {
        self.data.push_back(samples[0]);
        self.out = self.data.pop_front().unwrap();
    }

    fn read(&self) -> f32 {
        self.out
    }
}

pub fn delay(len: usize) -> Delay {
    Delay::new(len)
}

#[derive(Clone, Debug)]
pub struct Gain {
    gain: f32,
    out: f32,
}

impl Gain {
    fn new(gain: f32) -> Self {
        Gain { gain, out: 0.0 }
    }
}

impl Node for Gain {
    fn feed(&mut self, samples: &[f32]) {
        self.out = samples[0] * self.gain;
    }

    fn read(&self) -> f32 {
        self.out
    }
}

pub fn gain(gain: f32) -> Gain {
    Gain::new(gain)
}

#[derive(Clone, Debug)]
pub struct Constant {
    out: f32,
}

impl Constant {
    fn new(out: f32) -> Self {
        Constant { out }
    }
}

impl Node for Constant {
    fn feed(&mut self, _samples: &[f32]) {}

    fn read(&self) -> f32 {
        self.out
    }
}

pub fn constant(value: f32) -> Constant {
    Constant::new(value)
}

pub trait FnMutClone: for<'a> FnMut(&'a [f32]) -> f32 {
    fn clone_box(&self) -> Box<dyn FnMutClone>;
}

impl<F: for<'a> FnMut(&'a [f32]) -> f32 + Clone + 'static> FnMutClone for F {
    fn clone_box(&self) -> Box<dyn FnMutClone> {
        Box::new(self.clone())
    }
}

pub struct FNode {
    f: Box<dyn FnMutClone>,
    out: f32,
}

impl Clone for FNode {
    fn clone(&self) -> Self {
        FNode {
            f: self.f.clone_box(),
            out: self.out,
        }
    }
}

impl Debug for FNode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FNode")
            .field("f", &"[ommited]")
            .field("out", &self.out)
            .finish()
    }
}

impl FNode {
    fn new<F: for<'a> FnMut(&'a [f32]) -> f32 + Clone + 'static>(f: F) -> Self {
        FNode {
            f: Box::new(f),
            out: 0.0,
        }
    }
}

pub fn fnode<F: for<'a> FnMut(&'a [f32]) -> f32 + Clone + 'static>(f: F) -> FNode {
    FNode::new(f)
}

impl Node for FNode {
    fn feed(&mut self, samples: &[f32]) {
        self.out = (self.f)(samples);
    }

    fn read(&self) -> f32 {
        self.out
    }
}

#[derive(Clone, Debug)]
pub struct Dot {
    with: Vec<f32>,
    out: f32,
}

impl Dot {
    fn new(coeffs: impl Into<Vec<f32>>) -> Self {
        Dot {
            with: coeffs.into(),
            out: 0.0,
        }
    }
}

impl Node for Dot {
    fn feed(&mut self, samples: &[f32]) {
        assert_eq!(self.with.len(), samples.len());

        self.out = 0.0;
        for i in 0..self.with.len() {
            self.out += self.with[i] * samples[i];
        }
    }

    fn read(&self) -> f32 {
        self.out
    }
}

pub fn dot(coeffs: impl Into<Vec<f32>>) -> Dot {
    Dot::new(coeffs)
}
