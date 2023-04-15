use atomic_float::AtomicF32;
use eframe::egui::DragValue;
use serde::{Deserialize, Serialize};
use std::sync::atomic::Ordering;

use crate::compute::node::InputUi;

#[derive(Debug, Serialize, Deserialize)]
pub struct PositiveInput {
    s: AtomicF32,
}

impl PositiveInput {
    pub fn new(f: f32) -> Self {
        PositiveInput {
            s: AtomicF32::new(f),
        }
    }
}

impl InputUi for PositiveInput {
    fn show(&self, ui: &mut eframe::egui::Ui) {
        let mut s = self.s.load(Ordering::Acquire);
        let s_old = s;

        ui.add(
            DragValue::new(&mut s)
                .clamp_range(0.0..=999999.0)
                .fixed_decimals(if s_old < 1.0 {
                    2
                } else if s_old < 10.0 {
                    1
                } else {
                    0
                })
                .speed(if s_old < 1.0 {
                    0.01
                } else if s_old < 10.0 {
                    0.1
                } else {
                    1.0
                }),
        );

        self.s.store(s, Ordering::Release);
    }

    fn value(&self) -> f32 {
        self.s.load(Ordering::Relaxed)
    }
}
