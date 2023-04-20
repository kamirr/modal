use std::sync::atomic::Ordering;

use atomic_float::AtomicF32;
use eframe::egui;
use serde::{Deserialize, Serialize};

use crate::{compute::node::InputUi, serde_atomic_enum};

use super::real::RealInput;

#[atomic_enum::atomic_enum]
#[derive(PartialEq, Eq)]
pub enum TriggerMode {
    Up,
    Change,
}

serde_atomic_enum!(AtomicTriggerMode);

#[derive(Debug, Serialize, Deserialize)]
pub struct TriggerInput {
    mode: AtomicTriggerMode,
    level: RealInput,
    prev: AtomicF32,
}

impl TriggerInput {
    pub fn new(mode: TriggerMode, level: f32) -> Self {
        TriggerInput {
            mode: AtomicTriggerMode::new(mode),
            level: RealInput::new(level),
            prev: AtomicF32::new(0.0),
        }
    }

    pub fn level(&self) -> f32 {
        self.level.value(None)
    }
}

impl InputUi for TriggerInput {
    fn show_always(&self, ui: &mut eframe::egui::Ui) {
        let mut mode = self.mode.load(Ordering::Acquire);
        egui::ComboBox::from_label("")
            .selected_text(format!("{mode:?}"))
            .show_ui(ui, |ui| {
                ui.selectable_value(&mut mode, TriggerMode::Up, "Up");
                ui.selectable_value(&mut mode, TriggerMode::Change, "Change");
            });

        self.mode.store(mode, Ordering::Release);

        match mode {
            TriggerMode::Up => {
                self.level.show_disconnected(ui);
            }
            TriggerMode::Change => {}
        };
    }

    fn value(&self, recv: Option<f32>) -> f32 {
        let (prev, curr) = (self.prev.load(Ordering::Acquire), recv.unwrap_or(0.0));

        self.prev.store(curr, Ordering::Release);

        let emit = match self.mode.load(Ordering::Relaxed) {
            TriggerMode::Up => {
                let level = self.level.value(None);
                curr >= level && prev < level
            }
            TriggerMode::Change => curr != prev,
        };

        if emit {
            1.0
        } else {
            0.0
        }
    }
}