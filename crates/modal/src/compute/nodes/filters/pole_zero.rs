use atomic_enum::atomic_enum;
use serde::{Deserialize, Serialize};

use crate::{compute::inputs::slider::SliderInput, serde_atomic_enum, util::enum_combo_box};
use runtime::{ExternInputs, Value, ValueKind};

use crate::node::{Input, Node, NodeConfig, NodeEvent};

use std::{
    any::Any,
    sync::{atomic::Ordering, Arc},
};

#[atomic_enum]
#[derive(PartialEq, derive_more::Display, strum::EnumIter, Serialize, Deserialize)]
pub enum PoleZeroTy {
    Allpass,
    BlockDC,
}

serde_atomic_enum!(AtomicPoleZeroTy);

#[derive(Debug, Serialize, Deserialize)]
struct PoleZeroConfig {
    filt_ty: AtomicPoleZeroTy,
}

impl PoleZeroConfig {
    fn new(filt_ty: PoleZeroTy) -> Self {
        PoleZeroConfig {
            filt_ty: AtomicPoleZeroTy::new(filt_ty),
        }
    }
}

impl NodeConfig for PoleZeroConfig {
    fn show(&self, ui: &mut eframe::egui::Ui, _data: &dyn Any) {
        let mut filt_ty = self.filt_ty.load(Ordering::Acquire);

        enum_combo_box(ui, &mut filt_ty);

        self.filt_ty.store(filt_ty, Ordering::Release);
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RawPoleZero {
    pub a: [f32; 2],
    pub b: [f32; 2],
    pub gain: f32,
    in_hist: [f32; 2],
    out: f32,
}

impl Default for RawPoleZero {
    fn default() -> Self {
        RawPoleZero {
            a: [1.0, 0.0],
            b: [1.0, 0.0],
            gain: 1.0,
            in_hist: [0.0; 2],
            out: 0.0,
        }
    }
}

impl RawPoleZero {
    pub fn new(a: [f32; 2], b: [f32; 2]) -> Self {
        RawPoleZero {
            a,
            b,
            ..Default::default()
        }
    }

    pub fn feed(&mut self, input: f32) {
        self.in_hist = [self.in_hist[1], self.gain * input];

        self.out = (self.b[0] / self.a[0]) * self.in_hist[1]
            + (self.b[1] / self.a[0]) * self.in_hist[0]
            - (self.a[1] / self.a[0]) * self.out;
    }

    pub fn read(&self) -> f32 {
        self.out
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PoleZero {
    config: Arc<PoleZeroConfig>,
    coeff: Arc<SliderInput>,
    pole: Arc<SliderInput>,
    filt_ty: PoleZeroTy,
    raw: RawPoleZero,
}

impl PoleZero {
    pub fn new(ty: PoleZeroTy, coeff: f32, pole: f32) -> Self {
        let config = PoleZeroConfig::new(ty);
        let (a, b) = match ty {
            PoleZeroTy::Allpass => ([1.0, coeff], [coeff, 1.0]),
            PoleZeroTy::BlockDC => ([1.0, -pole], [1.0, -1.0]),
        };
        PoleZero {
            config: Arc::new(config),
            coeff: Arc::new(SliderInput::new(coeff, 0.0, 1.0)),
            pole: Arc::new(SliderInput::new(pole, 0.0, 1.0)),
            filt_ty: ty,
            raw: RawPoleZero::new(a, b),
        }
    }

    fn next(&mut self, input: f32, param: &Value) {
        let (a, b) = self.coeffs(param);
        self.raw.a = a;
        self.raw.b = b;
        self.raw.feed(input);
    }

    fn coeffs(&self, param: &Value) -> ([f32; 2], [f32; 2]) {
        let ty = self.config.filt_ty.load(Ordering::Relaxed);

        match ty {
            PoleZeroTy::Allpass => {
                let c = self.coeff.as_f32(param);
                ([1.0, c], [c, 1.0])
            }
            PoleZeroTy::BlockDC => {
                let p = self.pole.as_f32(param);
                ([1.0, -p], [1.0, -1.0])
            }
        }
    }
}

#[typetag::serde]
impl Node for PoleZero {
    fn feed(&mut self, _inputs: &ExternInputs, data: &[Value]) -> Vec<NodeEvent> {
        self.next(data[0].as_float().unwrap_or_default(), &data[1]);

        let new_filt_ty = self.config.filt_ty.load(Ordering::Relaxed);
        if self.filt_ty != new_filt_ty {
            self.filt_ty = new_filt_ty;

            vec![NodeEvent::RecalcInputs(self.inputs())]
        } else {
            Default::default()
        }
    }

    fn read(&self, out: &mut [Value]) {
        out[0] = Value::Float(self.raw.read())
    }
    fn config(&self) -> Option<Arc<dyn NodeConfig>> {
        Some(Arc::clone(&self.config) as Arc<_>)
    }

    fn inputs(&self) -> Vec<Input> {
        vec![
            Input::new("sig", ValueKind::Float),
            match &self.filt_ty {
                PoleZeroTy::Allpass => Input::stateful("c", &self.coeff),
                PoleZeroTy::BlockDC => Input::stateful("pole", &self.pole),
            },
        ]
    }
}

pub fn pole_zero() -> Box<dyn Node> {
    Box::new(PoleZero::new(PoleZeroTy::BlockDC, 0.5, 0.99))
}
