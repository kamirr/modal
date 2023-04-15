use atomic_float::AtomicF32;
use eframe::egui::DragValue;
use serde::Serialize;
use std::sync::atomic::Ordering;

use crate::compute::node::InputUi;

#[derive(Debug, Serialize)]
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
    fn show(&self, ui: &mut eframe::egui::Ui) {
        let mut f = self.f.load(Ordering::Acquire);

        ui.add(DragValue::new(&mut f));

        self.f.store(f, Ordering::Release);
    }

    fn value(&self) -> f32 {
        self.f.load(Ordering::Relaxed)
    }
}
