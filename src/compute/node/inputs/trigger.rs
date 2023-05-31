use std::sync::atomic::Ordering;

use atomic_float::AtomicF32;
use serde::{Deserialize, Serialize};

use crate::{
    compute::{node::InputUi, Value, ValueKind},
    serde_atomic_enum,
    util::enum_combo_box,
};

use super::real::RealInput;

#[atomic_enum::atomic_enum]
#[derive(PartialEq, Eq, derive_more::Display, strum::EnumIter)]
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

    pub fn trigger(&self, recv: &Value) -> bool {
        let (prev, curr) = (
            self.prev.load(Ordering::Acquire),
            recv.as_float().unwrap_or_default(),
        );

        self.prev.store(curr, Ordering::Release);

        match self.mode.load(Ordering::Relaxed) {
            TriggerMode::Up => {
                let level = self.level.get_f32(&Value::None);
                curr >= level && prev < level
            }
            TriggerMode::Change => curr != prev,
        }
    }
}

impl InputUi for TriggerInput {
    fn value_kind(&self) -> ValueKind {
        ValueKind::Float
    }

    fn show_always(&self, ui: &mut eframe::egui::Ui, verbose: bool) {
        let mut mode = self.mode.load(Ordering::Acquire);
        enum_combo_box(ui, &mut mode);
        self.mode.store(mode, Ordering::Release);

        if verbose {
            match mode {
                TriggerMode::Up => {
                    self.level.show_disconnected(ui, verbose);
                }
                TriggerMode::Change => {}
            };
        }
    }
}
