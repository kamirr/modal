use std::{collections::VecDeque, fmt::Debug};

use crate::param;

use super::{Node, NodeList, NodeMeta, Param, ParamValue};

#[derive(Clone, Debug)]
pub struct Placeholder;

impl Node for Placeholder {
    fn feed(&mut self, _samples: &[f32]) {
        unreachable!()
    }

    fn read(&self) -> f32 {
        unreachable!()
    }

    fn set_param(&mut self, _value: &[Param]) {
        unreachable!()
    }

    fn get_param(&self) -> Vec<Param> {
        unreachable!()
    }

    fn meta(&self) -> NodeMeta {
        NodeMeta::new::<&str, &str, _, _>([], [])
    }
}

#[derive(Clone, Debug)]
pub struct Add {
    out: f32,
    ins: u32,
}

impl Add {
    fn new(ins: u32) -> Self {
        Add { out: 0.0, ins }
    }
}

impl Node for Add {
    fn feed(&mut self, samples: &[f32]) {
        self.out = samples.iter().sum();
    }

    fn read(&self) -> f32 {
        self.out
    }

    fn set_param(&mut self, value: &[Param]) {
        self.ins = value[0].0[0].as_u();
    }

    fn get_param(&self) -> Vec<Param> {
        vec![Param(vec![ParamValue::U(self.ins)])]
    }

    fn meta(&self) -> NodeMeta {
        NodeMeta::new(
            (0..self.ins).map(|n| format!("sig {n}")),
            [("ins", param!(_ U))],
        )
    }
}

pub fn add() -> Add {
    Add::new(2)
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
}

impl Node for Delay {
    fn feed(&mut self, samples: &[f32]) {
        self.data.push_back(samples[0]);
        self.out = self.data.pop_front().unwrap();
    }

    fn read(&self) -> f32 {
        self.out
    }

    fn set_param(&mut self, value: &[Param]) {
        let len = value[0].0[0].as_u() as usize;
        while self.data.len() < len {
            self.data.push_back(0.0);
        }
        while self.data.len() > len {
            self.data.pop_back();
        }
    }

    fn get_param(&self) -> Vec<Param> {
        vec![Param(vec![ParamValue::U(self.data.len() as _)])]
    }

    fn meta(&self) -> NodeMeta {
        NodeMeta::new(["sig"], [("len", param!(_ U))])
    }
}

pub fn delay() -> Delay {
    Delay::new(4410)
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

    fn set_param(&mut self, value: &[Param]) {
        self.gain = value[0].0[0].as_f();
    }

    fn get_param(&self) -> Vec<Param> {
        vec![Param(vec![ParamValue::F(self.gain)])]
    }

    fn meta(&self) -> NodeMeta {
        NodeMeta::new(["sig"], [("gain", param!(_ F))])
    }
}

pub fn gain() -> Gain {
    Gain::new(1.0)
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
    fn read(&self) -> f32 {
        self.out
    }

    fn set_param(&mut self, value: &[Param]) {
        self.out = value[0].0[0].as_f();
    }

    fn get_param(&self) -> Vec<Param> {
        vec![Param(vec![ParamValue::F(self.out)])]
    }

    fn meta(&self) -> NodeMeta {
        NodeMeta::new::<&str, _, _, _>([], [("value", param!(_ F))])
    }
}

pub fn constant() -> Constant {
    Constant::new(0.0)
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

    fn set_param(&mut self, value: &[Param]) {
        self.with = value[0].0[0].as_fdyn().into();
    }

    fn get_param(&self) -> Vec<Param> {
        vec![Param(vec![ParamValue::FDyn(self.with.clone())])]
    }

    fn meta(&self) -> NodeMeta {
        NodeMeta::new(
            (0..self.with.len()).map(|i| format!("sig {i}")),
            [("coeffs", param!(_ FDyn))],
        )
    }
}

pub fn dot() -> Dot {
    Dot::new([1.0])
}

#[derive(Clone, Debug)]
pub struct Sine {
    t: f32,
    step: f32,
}

impl Sine {
    fn new(hz: u32) -> Self {
        Sine {
            t: 0.0,
            step: (hz as f32) * Self::hz_to_dt(),
        }
    }

    fn hz_to_dt() -> f32 {
        2.0 * std::f32::consts::PI / 44100.0
    }
}

impl Node for Sine {
    fn feed(&mut self, _samples: &[f32]) {
        self.t = (self.t + self.step) % (2.0 * std::f32::consts::PI);
    }

    fn read(&self) -> f32 {
        self.t.sin()
    }

    fn set_param(&mut self, value: &[Param]) {
        self.step = value[0].0[0].as_u() as f32 * Self::hz_to_dt();
    }

    fn get_param(&self) -> Vec<Param> {
        vec![Param(vec![ParamValue::U(
            (self.step / Self::hz_to_dt()).round() as u32,
        )])]
    }

    fn meta(&self) -> NodeMeta {
        NodeMeta::new::<&str, _, _, _>([], [("freq", param!(_ F))])
    }
}

fn sine() -> Sine {
    Sine::new(440)
}

pub struct Basic;

impl NodeList for Basic {
    fn all(&self) -> Vec<(fn() -> Box<dyn Node>, &'static str)> {
        vec![
            (|| Box::new(add()), "Add"),
            (|| Box::new(constant()), "Constant"),
            (|| Box::new(delay()), "Delay"),
            (|| Box::new(dot()), "Dot"),
            (|| Box::new(gain()), "Gain"),
            (|| Box::new(sine()), "Sine"),
        ]
    }
}
