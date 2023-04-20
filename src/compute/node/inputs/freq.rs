use atomic_float::AtomicF32;
use eframe::egui::DragValue;
use serde::{Deserialize, Serialize};
use std::sync::atomic::Ordering;

use crate::compute::node::InputUi;

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
}

impl InputUi for FreqInput {
    fn show_disconnected(&self, ui: &mut eframe::egui::Ui) {
        let mut f = self.f.load(Ordering::Acquire);

        ui.add(DragValue::new(&mut f));

        self.f.store(f, Ordering::Release);
    }

    fn value(&self, recv: Option<f32>) -> f32 {
        recv.unwrap_or(self.f.load(Ordering::Relaxed))
    }
}
