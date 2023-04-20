use atomic_float::AtomicF32;
use eframe::egui;
use serde::{Deserialize, Serialize};
use std::sync::atomic::Ordering;

use crate::compute::node::InputUi;

#[derive(Debug, Serialize, Deserialize)]
pub struct SliderInput {
    s: AtomicF32,
    min: f32,
    max: f32,
}

impl SliderInput {
    pub fn new(f: f32, min: f32, max: f32) -> Self {
        SliderInput {
            s: AtomicF32::new(f),
            min,
            max,
        }
    }
}

impl InputUi for SliderInput {
    fn show_disconnected(&self, ui: &mut eframe::egui::Ui) {
        let mut s = self.s.load(Ordering::Acquire);

        ui.add(egui::Slider::new(&mut s, self.min..=self.max));

        self.s.store(s, Ordering::Release);
    }

    fn value(&self) -> f32 {
        self.s.load(Ordering::Relaxed)
    }
}
