use atomic_enum::atomic_enum;
use atomic_float::AtomicF32;
use eframe::egui::{ComboBox, DragValue};
use serde::{Deserialize, Serialize};

use super::{Input, Node, NodeConfig, NodeEvent, NodeList};
use std::{
    f32::consts::PI,
    sync::{
        atomic::{AtomicU32, AtomicUsize, Ordering},
        Arc,
    },
};

#[atomic_enum]
#[derive(PartialEq)]
enum BiquadTy {
    Lpf,
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

#[derive(Debug, Serialize, Deserialize)]
struct BiquadConfig {
    ty: AtomicBiquadTy,
    f0: AtomicU32,
    q: AtomicF32,
}

impl BiquadConfig {
    fn new(ty: BiquadTy, f0: u32, q: f32) -> Self {
        BiquadConfig {
            ty: AtomicBiquadTy::new(ty),
            f0: AtomicU32::new(f0),
            q: AtomicF32::new(q),
        }
    }

    fn coeffs(&self) -> ([f32; 3], [f32; 3]) {
        let ty = self.ty.load(Ordering::Relaxed);
        let f0 = self.f0.load(Ordering::Relaxed);
        let q = self.q.load(Ordering::Relaxed);

        let w0 = 2.0 * PI * (f0 as f32) / 44100.0;
        let alpha = w0.sin() / 2.0 / q;

        match ty {
            BiquadTy::Lpf => (
                [1.0 + alpha, -2.0 * w0.cos(), 1.0 - alpha],
                [
                    (1.0 - w0.cos()) / 2.0,
                    1.0 - w0.cos(),
                    (1.0 - w0.cos()) / 2.0,
                ],
            ),
        }
    }
}

impl NodeConfig for BiquadConfig {
    fn show(&self, ui: &mut eframe::egui::Ui) {
        let mut ty = self.ty.load(Ordering::Acquire);
        let mut f0 = self.f0.load(Ordering::Acquire);
        let mut q = self.q.load(Ordering::Acquire);

        ComboBox::from_label("")
            .selected_text(format!("{ty:?}"))
            .show_ui(ui, |ui| {
                ui.selectable_value(&mut ty, BiquadTy::Lpf, "Lpf");
            });
        ui.add(DragValue::new(&mut f0).clamp_range(0.0..=20000.0));
        ui.add(DragValue::new(&mut q).clamp_range(0.1..=10.0));

        self.ty.store(ty, Ordering::Release);
        self.f0.store(f0, Ordering::Release);
        self.q.store(q, Ordering::Release);
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct Biquad {
    config: Arc<BiquadConfig>,
    in_hist: [f32; 3],
    out_hist: [f32; 2],
}

impl Biquad {
    fn new() -> Self {
        let config = BiquadConfig::new(BiquadTy::Lpf, 440, 1.0);
        Biquad {
            config: Arc::new(config),
            in_hist: [0.0; 3],
            out_hist: [0.0; 2],
        }
    }

    fn next(&mut self, input: f32) {
        let (a, b) = self.config.coeffs();

        self.in_hist = [self.in_hist[1], self.in_hist[2], input];
        let out = (b[0] / a[0]) * self.in_hist[2]
            + (b[1] / a[0]) * self.in_hist[1]
            + (b[2] / a[0]) * self.in_hist[0]
            - (a[1] / a[0]) * self.out_hist[1]
            - (a[2] / a[0]) * self.out_hist[0];

        self.out_hist = [self.out_hist[1], out];
    }
}

#[typetag::serde]
impl Node for Biquad {
    fn feed(&mut self, data: &[Option<f32>]) -> Vec<NodeEvent> {
        self.next(data[0].unwrap_or(0.0));

        Default::default()
    }

    fn read(&self) -> f32 {
        self.out_hist[1]
    }

    fn config(&self) -> Option<Arc<dyn NodeConfig>> {
        Some(Arc::clone(&self.config) as Arc<_>)
    }

    fn inputs(&self) -> Vec<Input> {
        vec![Input::new("sig")]
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
