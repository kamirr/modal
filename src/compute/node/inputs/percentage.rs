use atomic_float::AtomicF32;
use eframe::egui::DragValue;
use serde::{Deserialize, Serialize};
use std::sync::atomic::Ordering;

use crate::compute::{node::InputUi, Value, ValueKind};

#[derive(Debug, Serialize, Deserialize)]
pub struct PercentageInput {
    s: AtomicF32,
}

impl PercentageInput {
    pub fn new(f: f32) -> Self {
        PercentageInput {
            s: AtomicF32::new(f),
        }
    }

    pub fn get_f32(&self, recv: &Value) -> f32 {
        recv.as_float().unwrap_or(self.s.load(Ordering::Relaxed)) / 100.0
    }
}

impl InputUi for PercentageInput {
    fn value_kind(&self) -> ValueKind {
        ValueKind::Float
    }

    fn show_disconnected(&self, ui: &mut eframe::egui::Ui, _verbose: bool) {
        let mut s = self.s.load(Ordering::Acquire);

        ui.horizontal(|ui| {
            ui.add(
                DragValue::new(&mut s)
                    .clamp_range(0.0..=100.0)
                    .fixed_decimals(0)
                    .speed(1),
            );
            ui.label("%");
        });

        self.s.store(s, Ordering::Release);
    }
}
