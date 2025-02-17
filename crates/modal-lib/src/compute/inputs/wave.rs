use atomic_float::AtomicF32;
use eframe::epaint::ColorImage;
use serde::{Deserialize, Serialize};
use std::sync::atomic::Ordering;

use crate::util::load_image_from_path;
use runtime::{node::InputUi, Value, ValueKind};

#[derive(Debug, Serialize, Deserialize)]
pub struct WaveInput {
    s: AtomicF32,
}

impl WaveInput {
    pub fn new(f: f32) -> Self {
        WaveInput {
            s: AtomicF32::new(f),
        }
    }

    pub fn as_f32(&self, recv: &Value) -> f32 {
        recv.as_float().unwrap_or(self.s.load(Ordering::Relaxed))
    }

    fn wave_knob_image() -> ColorImage {
        load_image_from_path(include_bytes!("assets/knob.png"))
    }

    fn wave_scale_image() -> ColorImage {
        load_image_from_path(include_bytes!("assets/shape-scale.png"))
    }
}

impl InputUi for WaveInput {
    fn value_kind(&self) -> ValueKind {
        ValueKind::Float
    }

    fn show_disconnected(&self, ui: &mut eframe::egui::Ui, _verbose: bool) {
        let mut s = self.s.load(Ordering::Acquire);

        ui.add(egui_knobs::Knob::new(
            &mut s,
            Self::wave_knob_image,
            Self::wave_scale_image,
        ));

        self.s.store(s, Ordering::Release);
    }
}
