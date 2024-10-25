use atomic_float::AtomicF32;
use eframe::egui::DragValue;
use serde::{Deserialize, Serialize};
use std::sync::atomic::Ordering;

use crate::compute::{node::InputUi, Value, ValueKind};

#[derive(Debug, Serialize, Deserialize)]
pub struct RealInput {
    s: AtomicF32,
}

impl RealInput {
    pub fn new(f: f32) -> Self {
        RealInput {
            s: AtomicF32::new(f),
        }
    }

    pub fn get_f32(&self, recv: &Value) -> f32 {
        recv.as_float().unwrap_or(self.s.load(Ordering::Relaxed))
    }
}

impl InputUi for RealInput {
    fn value_kind(&self) -> ValueKind {
        ValueKind::Float
    }

    fn show_disconnected(&self, ui: &mut eframe::egui::Ui, _verbose: bool) {
        let mut s = self.s.load(Ordering::Acquire);
        let s_old = s;

        ui.add(
            DragValue::new(&mut s)
                .range(-999999.0..=999999.0)
                .fixed_decimals(if s_old.abs() < 1.0 {
                    2
                } else if s_old.abs() < 10.0 {
                    1
                } else {
                    0
                })
                .speed(if s_old.abs() < 1.0 {
                    0.01
                } else if s_old.abs() < 10.0 {
                    0.1
                } else {
                    1.0
                }),
        );

        self.s.store(s, Ordering::Release);
    }
}
