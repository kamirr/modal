use atomic_float::AtomicF32;
use eframe::egui::DragValue;
use serde::{Deserialize, Serialize};
use std::sync::atomic::Ordering;

use runtime::{node::InputUi, Value, ValueKind};

#[derive(Debug, Serialize, Deserialize)]
pub struct FreqInput {
    f: AtomicF32,
    min: i32,
    max: i32,
}

impl FreqInput {
    pub fn new(f: f32) -> Self {
        FreqInput {
            f: AtomicF32::new(f),
            min: 0,
            max: 22050,
        }
    }

    pub fn min(mut self, min: i32) -> Self {
        self.min = min;
        self
    }

    pub fn max(mut self, max: i32) -> Self {
        self.max = max;
        self
    }

    pub fn get_f32(&self, recv: &Value) -> f32 {
        recv.as_float().unwrap_or(self.f.load(Ordering::Relaxed))
    }
}

impl InputUi for FreqInput {
    fn value_kind(&self) -> ValueKind {
        ValueKind::Float
    }

    fn show_disconnected(&self, ui: &mut eframe::egui::Ui, _verbose: bool) {
        let mut f = self.f.load(Ordering::Acquire);

        ui.add(DragValue::new(&mut f).range(self.min..=self.max));

        self.f.store(f, Ordering::Release);
    }
}
