use std::sync::atomic::{AtomicBool, Ordering};

use atomic_float::AtomicF32;
use serde::{Deserialize, Serialize};

use crate::{
    compute::{node::InputUi, Value, ValueKind},
    serde_atomic_enum,
    util::enum_combo_box,
};

use super::{beat::BeatInput, real::RealInput};

#[atomic_enum::atomic_enum]
#[derive(PartialEq, Eq, derive_more::Display, strum::EnumIter)]
pub enum TriggerMode {
    Up,
    Down,
    Change,
    Beat,
}

serde_atomic_enum!(AtomicTriggerMode);

#[derive(Debug, Serialize, Deserialize)]
pub struct TriggerInput {
    mode: AtomicTriggerMode,
    level: RealInput,
    beat: BeatInput,
    prev: AtomicF32,
    need_update: AtomicBool,
    force_trigger: AtomicBool,
}

impl TriggerInput {
    pub fn new(mode: TriggerMode, level: f32) -> Self {
        TriggerInput {
            mode: AtomicTriggerMode::new(mode),
            level: RealInput::new(level),
            beat: BeatInput::new(true),
            prev: AtomicF32::new(0.0),
            need_update: AtomicBool::new(false),
            force_trigger: AtomicBool::new(false),
        }
    }

    pub fn trigger(&self, recv: &Value) -> bool {
        let (prev, curr) = (
            self.prev.load(Ordering::Acquire),
            recv.as_float().unwrap_or_default(),
        );

        self.prev.store(curr, Ordering::Release);

        let do_trigger = match self.mode.load(Ordering::Relaxed) {
            TriggerMode::Up => {
                let level = self.level.get_f32(&Value::None);
                curr >= level && prev < level
            }
            TriggerMode::Down => {
                let level = self.level.get_f32(&Value::None);
                curr <= level && prev > level
            }
            TriggerMode::Beat => self.beat.process(recv).is_some(),
            TriggerMode::Change => curr != prev,
        };
        let force_trigger = self.force_trigger.swap(false, Ordering::Relaxed);

        do_trigger || force_trigger
    }
}

impl InputUi for TriggerInput {
    fn value_kind(&self) -> ValueKind {
        use TriggerMode::*;
        match self.mode.load(Ordering::Relaxed) {
            Up | Down | Change => ValueKind::Float,
            Beat => ValueKind::Beat,
        }
    }

    fn show_name(&self, ui: &mut eframe::egui::Ui, name: &str) {
        if ui.button(name).clicked() {
            self.force_trigger.store(true, Ordering::Relaxed);
        }
    }

    fn show_always(&self, ui: &mut eframe::egui::Ui, verbose: bool) {
        let mut mode = self.mode.load(Ordering::Acquire);
        let old_mode = mode;

        if verbose {
            match mode {
                TriggerMode::Up | TriggerMode::Down => {
                    ui.horizontal(|ui| {
                        enum_combo_box(ui, &mut mode);
                        self.level.show_disconnected(ui, verbose);
                    });
                }
                TriggerMode::Beat => {
                    ui.vertical(|ui| {
                        enum_combo_box(ui, &mut mode);
                        self.beat.show_always(ui, verbose);
                    });
                }
                TriggerMode::Change => {
                    enum_combo_box(ui, &mut mode);
                }
            };
        }

        if mode != old_mode {
            self.need_update.store(true, Ordering::Relaxed);
        }

        self.mode.store(mode, Ordering::Release);
    }

    fn needs_deep_update(&self) -> bool {
        self.need_update.swap(false, Ordering::Relaxed)
    }
}
