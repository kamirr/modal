use atomic_float::AtomicF32;
use eframe::egui::DragValue;
use serde::{Deserialize, Serialize};
use std::sync::atomic::Ordering;

use crate::compute::{node::InputUi, Value};

#[derive(Debug, Serialize, Deserialize)]
pub struct FreqInput {
    f: AtomicF32,
}

impl FreqInput {
    pub fn new(f: f32) -> Self {
        FreqInput {
            f: AtomicF32::new(f),
        }
    }

    pub fn get_f32(&self, recv: &Value) -> f32 {
        recv.as_float().unwrap_or(self.f.load(Ordering::Relaxed))
    }
}

impl InputUi for FreqInput {
    fn show_disconnected(&self, ui: &mut eframe::egui::Ui, _verbose: bool) {
        let mut f = self.f.load(Ordering::Acquire);

        ui.add(DragValue::new(&mut f));

        self.f.store(f, Ordering::Release);
    }
}
