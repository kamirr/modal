use atomic_float::AtomicF32;
use eframe::egui::DragValue;
use serde::{Deserialize, Serialize};
use std::sync::atomic::Ordering;

use crate::compute::node::InputUi;

#[derive(Debug, Serialize, Deserialize)]
pub struct SigInput {
    s: AtomicF32,
}

impl SigInput {
    pub fn new(f: f32) -> Self {
        SigInput {
            s: AtomicF32::new(f),
        }
    }
}

impl InputUi for SigInput {
    fn show(&self, ui: &mut eframe::egui::Ui) {
        let mut s = self.s.load(Ordering::Acquire);

        ui.add(
            DragValue::new(&mut s)
                .clamp_range(-1.0..=1.0)
                .fixed_decimals(2)
                .speed(0.02),
        );

        self.s.store(s, Ordering::Release);
    }

    fn value(&self) -> f32 {
        self.s.load(Ordering::Relaxed)
    }
}
