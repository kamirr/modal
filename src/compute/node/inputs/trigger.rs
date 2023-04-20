use std::sync::atomic::Ordering;

use atomic_float::AtomicF32;
use serde::{Deserialize, Serialize};

use crate::compute::node::InputUi;

use super::real::RealInput;

#[derive(Debug, Serialize, Deserialize)]
pub struct TriggerInput {
    level: RealInput,
    prev: AtomicF32,
}

impl TriggerInput {
    pub fn new(level: f32) -> Self {
        TriggerInput {
            level: RealInput::new(level),
            prev: AtomicF32::new(0.0),
        }
    }
}

impl InputUi for TriggerInput {
    fn show_always(&self, ui: &mut eframe::egui::Ui) {
        self.level.show_disconnected(ui);
    }

    fn value(&self, recv: Option<f32>) -> f32 {
        let level = self.level.value(None);

        let (prev, curr) = (self.prev.load(Ordering::Acquire), recv.unwrap_or(0.0));

        self.prev.store(curr, Ordering::Release);

        if curr >= level && prev < level {
            1.0
        } else {
            0.0
        }
    }
}
