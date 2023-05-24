use atomic_float::AtomicF32;
use eframe::egui;
use serde::{Deserialize, Serialize};
use std::sync::atomic::Ordering;

use crate::compute::{node::InputUi, Value};

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
}

impl InputUi for WaveInput {
    fn show_disconnected(&self, ui: &mut eframe::egui::Ui, _verbose: bool) {
        let mut s = self.s.load(Ordering::Acquire);

        ui.add(egui::Slider::new(&mut s, 0.0..=1.0));

        self.s.store(s, Ordering::Release);
    }
}
