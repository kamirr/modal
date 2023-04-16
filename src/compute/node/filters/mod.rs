use atomic_enum::atomic_enum;
use eframe::egui::ComboBox;
use serde::{Deserialize, Serialize};

use super::{
    inputs::{freq::FreqInput, positive::PositiveInput},
    Input, InputUi, Node, NodeConfig, NodeEvent, NodeList,
};
use std::{
    f32::consts::PI,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
};

#[atomic_enum]
#[derive(PartialEq)]
enum BiquadTy {
    Lpf,
    Hpf,
    Bpf,
    Notch,
}

impl Serialize for AtomicBiquadTy {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.0.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for AtomicBiquadTy {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        AtomicUsize::deserialize(deserializer).map(|inner| AtomicBiquadTy(inner))
    }
}

#[atomic_enum]
#[derive(PartialEq, Serialize, Deserialize)]
enum ParamTy {
    Q,
    Bw,
}

impl Serialize for AtomicParamTy {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.0.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for AtomicParamTy {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        AtomicUsize::deserialize(deserializer).map(|inner| AtomicParamTy(inner))
    }
}

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
    fn show(&self, ui: &mut eframe::egui::Ui) {
        let mut filt_ty = self.filt_ty.load(Ordering::Acquire);
        let mut param_ty = self.param_ty.load(Ordering::Acquire);

        ComboBox::from_id_source("foo")
            .selected_text(format!("{filt_ty:?}"))
            .show_ui(ui, |ui| {
                ui.selectable_value(&mut filt_ty, BiquadTy::Lpf, "Lpf");
                ui.selectable_value(&mut filt_ty, BiquadTy::Hpf, "Hpf");
                ui.selectable_value(&mut filt_ty, BiquadTy::Bpf, "Bpf");
                ui.selectable_value(&mut filt_ty, BiquadTy::Notch, "Notch");
            });
        ComboBox::from_id_source("bar")
            .selected_text(format!("{param_ty:?}"))
            .show_ui(ui, |ui| {
                ui.selectable_value(&mut param_ty, ParamTy::Q, "Q");
                ui.selectable_value(&mut param_ty, ParamTy::Bw, "BW");
            });

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

    fn next(&mut self, input: f32, f0: Option<f32>, param: Option<f32>) {
        let (a, b) = self.coeffs(f0, param);

        self.in_hist = [self.in_hist[1], self.in_hist[2], input];
        let out = (b[0] / a[0]) * self.in_hist[2]
            + (b[1] / a[0]) * self.in_hist[1]
            + (b[2] / a[0]) * self.in_hist[0]
            - (a[1] / a[0]) * self.out_hist[1]
            - (a[2] / a[0]) * self.out_hist[0];

        self.out_hist = [self.out_hist[1], out];
    }

    fn coeffs(&self, f0: Option<f32>, param: Option<f32>) -> ([f32; 3], [f32; 3]) {
        let ty = self.config.filt_ty.load(Ordering::Relaxed);
        let param_ty = self.config.param_ty.load(Ordering::Relaxed);

        let f0 = f0.unwrap_or(self.f0.value());
        let param = param.unwrap_or(match param_ty {
            ParamTy::Q => self.q.value(),
            ParamTy::Bw => self.bw.value(),
        });

        let w0 = 2.0 * PI * (f0 as f32) / 44100.0;
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
            BiquadTy::Notch => (
                [1.0 + alpha, -2.0 * w0cos, 1.0 - alpha],
                [1.0, -2.0 * w0cos, 1.0],
            ),
        }
    }
}

#[typetag::serde]
impl Node for Biquad {
    fn feed(&mut self, data: &[Option<f32>]) -> Vec<NodeEvent> {
        self.next(data[0].unwrap_or(0.0), data[1], data[2]);

        let new_param_ty = self.config.param_ty.load(Ordering::Relaxed);
        if self.param_ty != new_param_ty {
            self.param_ty = new_param_ty;

            vec![NodeEvent::RecalcInputs(self.inputs())]
        } else {
            Default::default()
        }
    }

    fn read(&self) -> f32 {
        self.out_hist[1]
    }

    fn config(&self) -> Option<Arc<dyn NodeConfig>> {
        Some(Arc::clone(&self.config) as Arc<_>)
    }

    fn inputs(&self) -> Vec<Input> {
        vec![
            Input::new("sig"),
            Input::with_default("f0", &self.f0),
            match &self.param_ty {
                ParamTy::Q => Input::with_default("Q", &self.q),
                ParamTy::Bw => Input::with_default("BW", &self.bw),
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
