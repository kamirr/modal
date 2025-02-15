use atomic_float::AtomicF32;
use eframe::egui::DragValue;
use serde::{Deserialize, Serialize};
use std::{f32::consts::PI, sync::atomic::Ordering};

use crate::compute::{node::InputUi, Value, ValueKind};

#[derive(Debug, Serialize, Deserialize)]
pub struct AngleInput {
    s: AtomicF32,
}

impl AngleInput {
    pub fn new(f: f32) -> Self {
        AngleInput {
            s: AtomicF32::new(f),
        }
    }

    pub fn degrees(&self, recv: &Value) -> f32 {
        recv.as_float().unwrap_or(self.s.load(Ordering::Relaxed))
    }

    pub fn radians(&self, recv: &Value) -> f32 {
        self.degrees(recv) / 180.0 * PI
    }
}

impl InputUi for AngleInput {
    fn value_kind(&self) -> ValueKind {
        ValueKind::Float
    }

    fn show_disconnected(&self, ui: &mut eframe::egui::Ui, _verbose: bool) {
        let mut s = self.s.load(Ordering::Acquire);

        ui.horizontal(|ui| {
            ui.add(
                DragValue::new(&mut s)
                    .range(0.0..=360.0)
                    .fixed_decimals(0)
                    .speed(1),
            );
            ui.label("°");
        });

        self.s.store(s, Ordering::Release);
    }
}
