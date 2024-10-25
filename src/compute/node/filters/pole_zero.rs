use atomic_enum::atomic_enum;
use serde::{Deserialize, Serialize};

use crate::{
    compute::{node::inputs::slider::SliderInput, Value, ValueKind},
    serde_atomic_enum,
    util::enum_combo_box,
};

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
pub struct PoleZero {
    config: Arc<PoleZeroConfig>,
    coeff: Arc<SliderInput>,
    pole: Arc<SliderInput>,
    filt_ty: PoleZeroTy,
    in_hist: [f32; 2],
    out_hist: [f32; 2],
}

impl PoleZero {
    pub fn new(ty: PoleZeroTy, coeff: f32, pole: f32) -> Self {
        let config = PoleZeroConfig::new(ty);
        PoleZero {
            config: Arc::new(config),
            coeff: Arc::new(SliderInput::new(coeff, 0.0, 1.0)),
            pole: Arc::new(SliderInput::new(pole, 0.0, 1.0)),
            filt_ty: ty,
            in_hist: [0.0; 2],
            out_hist: [0.0; 2],
        }
    }

    fn next(&mut self, input: f32, param: &Value) {
        let (a, b) = self.coeffs(param);

        self.in_hist = [self.in_hist[1], input];
        let out = (b[0] / a[0]) * self.in_hist[1] + (b[1] / a[0]) * self.in_hist[0]
            - (a[1] / a[0]) * self.out_hist[0];

        self.out_hist = [self.out_hist[1], out];
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
    fn feed(&mut self, data: &[Value]) -> Vec<NodeEvent> {
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
        out[0] = Value::Float(self.out_hist[1])
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
