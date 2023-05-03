use atomic_enum::atomic_enum;
use serde::{Deserialize, Serialize};

use crate::{
    compute::{Value, ValueDiscriminants},
    serde_atomic_enum,
    util::enum_combo_box,
};

use super::{
    inputs::{freq::FreqInput, positive::PositiveInput},
    Input, Node, NodeConfig, NodeEvent, NodeList,
};
use std::{
    any::Any,
    f32::consts::PI,
    sync::{atomic::Ordering, Arc},
};

#[atomic_enum]
#[derive(PartialEq, derive_more::Display, strum::EnumIter)]
enum BiquadTy {
    Lpf,
    Hpf,
    Bpf,
    Apf,
    Notch,
}

serde_atomic_enum!(AtomicBiquadTy);

#[atomic_enum]
#[derive(PartialEq, Serialize, Deserialize, derive_more::Display, strum::EnumIter)]
enum ParamTy {
    Q,
    Bw,
}

serde_atomic_enum!(AtomicParamTy);

#[derive(Debug, Serialize, Deserialize)]
struct BiquadConfig {
    filt_ty: AtomicBiquadTy,
    param_ty: AtomicParamTy,
}

impl BiquadConfig {
    fn new(filt_ty: BiquadTy, param_ty: ParamTy) -> Self {
        BiquadConfig {
            filt_ty: AtomicBiquadTy::new(filt_ty),
            param_ty: AtomicParamTy::new(param_ty),
        }
    }
}

impl NodeConfig for BiquadConfig {
    fn show(&self, ui: &mut eframe::egui::Ui, _data: &dyn Any) {
        let mut filt_ty = self.filt_ty.load(Ordering::Acquire);
        let mut param_ty = self.param_ty.load(Ordering::Acquire);

        enum_combo_box(ui, &mut filt_ty);
        enum_combo_box(ui, &mut param_ty);

        self.filt_ty.store(filt_ty, Ordering::Release);
        self.param_ty.store(param_ty, Ordering::Release);
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct Biquad {
    config: Arc<BiquadConfig>,
    f0: Arc<FreqInput>,
    q: Arc<PositiveInput>,
    bw: Arc<PositiveInput>,
    param_ty: ParamTy,
    in_hist: [f32; 3],
    out_hist: [f32; 2],
}

impl Biquad {
    fn new() -> Self {
        let config = BiquadConfig::new(BiquadTy::Lpf, ParamTy::Q);
        Biquad {
            config: Arc::new(config),
            f0: Arc::new(FreqInput::new(440.0)),
            q: Arc::new(PositiveInput::new(0.707)),
            bw: Arc::new(PositiveInput::new(1.0)),
            param_ty: ParamTy::Q,
            in_hist: [0.0; 3],
            out_hist: [0.0; 2],
        }
    }

    fn next(&mut self, input: f32, f0: &Value, param: &Value) {
        let (a, b) = self.coeffs(f0, param);

        self.in_hist = [self.in_hist[1], self.in_hist[2], input];
        let out = (b[0] / a[0]) * self.in_hist[2]
            + (b[1] / a[0]) * self.in_hist[1]
            + (b[2] / a[0]) * self.in_hist[0]
            - (a[1] / a[0]) * self.out_hist[1]
            - (a[2] / a[0]) * self.out_hist[0];

        self.out_hist = [self.out_hist[1], out];
    }

    fn coeffs(&self, f0: &Value, param: &Value) -> ([f32; 3], [f32; 3]) {
        let ty = self.config.filt_ty.load(Ordering::Relaxed);
        let param_ty = self.config.param_ty.load(Ordering::Relaxed);

        let f0 = self.f0.get_f32(f0);
        let param = match param_ty {
            ParamTy::Q => self.q.get_f32(param),
            ParamTy::Bw => self.bw.get_f32(param),
        };

        let w0 = 2.0 * PI * f0 / 44100.0;
        let w0sin = w0.sin();
        let w0cos = w0.cos();

        let alpha = match param_ty {
            ParamTy::Q => w0sin / 2.0 / param,
            ParamTy::Bw => w0sin * (2f32.ln() / 2.0 * param * w0 / w0sin).sinh(),
        };

        match ty {
            BiquadTy::Lpf => (
                [1.0 + alpha, -2.0 * w0cos, 1.0 - alpha],
                [(1.0 - w0cos) / 2.0, 1.0 - w0cos, (1.0 - w0cos) / 2.0],
            ),
            BiquadTy::Hpf => (
                [1.0 + alpha, -2.0 * w0cos, 1.0 - alpha],
                [(1.0 + w0cos) / 2.0, -1.0 - w0cos, (1.0 + w0cos) / 2.0],
            ),
            BiquadTy::Bpf => (
                [1.0 + alpha, -2.0 * w0cos, 1.0 - alpha],
                [alpha, 0.0, -alpha],
            ),
            BiquadTy::Apf => (
                [1.0 + alpha, -2.0 * w0cos, 1.0 - alpha],
                [1.0 - alpha, -2.0 * w0cos, 1.0 + alpha],
            ),
            BiquadTy::Notch => (
                [1.0 + alpha, -2.0 * w0cos, 1.0 - alpha],
                [1.0, -2.0 * w0cos, 1.0],
            ),
        }
    }
}

#[typetag::serde]
impl Node for Biquad {
    fn feed(&mut self, data: &[Value]) -> Vec<NodeEvent> {
        self.next(data[0].as_float().unwrap_or_default(), &data[1], &data[2]);

        let new_param_ty = self.config.param_ty.load(Ordering::Relaxed);
        if self.param_ty != new_param_ty {
            self.param_ty = new_param_ty;

            vec![NodeEvent::RecalcInputs(self.inputs())]
        } else {
            Default::default()
        }
    }

    fn read(&self) -> Value {
        Value::Float(self.out_hist[1])
    }

    fn config(&self) -> Option<Arc<dyn NodeConfig>> {
        Some(Arc::clone(&self.config) as Arc<_>)
    }

    fn inputs(&self) -> Vec<Input> {
        vec![
            Input::new("sig", ValueDiscriminants::Float),
            Input::with_default("f0", ValueDiscriminants::Float, &self.f0),
            match &self.param_ty {
                ParamTy::Q => Input::with_default("Q", ValueDiscriminants::Float, &self.q),
                ParamTy::Bw => Input::with_default("BW", ValueDiscriminants::Float, &self.bw),
            },
        ]
    }
}

fn biquad() -> Box<dyn Node> {
    Box::new(Biquad::new())
}

pub struct Filters;

impl NodeList for Filters {
    fn all(&self) -> Vec<(Box<dyn Node>, String)> {
        vec![(biquad(), "BiQuad Filter".into())]
    }
}
