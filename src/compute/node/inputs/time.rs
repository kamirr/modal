use std::sync::atomic::Ordering;

use atomic_float::AtomicF32;
use eframe::egui;
use serde::{Deserialize, Serialize};

use crate::{
    compute::{node::InputUi, Value, ValueKind},
    serde_atomic_enum,
    util::enum_combo_box,
};

use super::positive::PositiveInput;

#[atomic_enum::atomic_enum]
#[derive(PartialEq, Eq, derive_more::Display, strum::EnumIter)]
enum TimeUnit {
    Samples,
    Seconds,
    Miliseconds,
}

serde_atomic_enum!(AtomicTimeUnit);

#[derive(Debug, Serialize, Deserialize)]
pub struct TimeInput {
    samples: AtomicF32,
    in_ty: AtomicTimeUnit,
}

impl TimeInput {
    pub fn new(samples: f32) -> Self {
        TimeInput {
            samples: AtomicF32::new(samples),
            in_ty: AtomicTimeUnit::new(TimeUnit::Miliseconds),
        }
    }

    pub fn from_ms(ms: f32) -> Self {
        TimeInput {
            samples: AtomicF32::new(ms * 44100.0 / 1000.0),
            in_ty: AtomicTimeUnit::new(TimeUnit::Miliseconds),
        }
    }

    pub fn get_samples(&self, recv: &Value) -> f32 {
        recv.as_float()
            .unwrap_or(self.samples.load(Ordering::Relaxed))
    }

    pub fn get_ms(&self, recv: &Value) -> f32 {
        self.get_samples(recv) as f32 / 44100.0 * 1000.0
    }
}

impl InputUi for TimeInput {
    fn value_kind(&self) -> ValueKind {
        ValueKind::Float
    }

    fn show_always(&self, ui: &mut egui::Ui, verbose: bool) {
        if verbose {
            let mut ty = self.in_ty.load(Ordering::Acquire);

            enum_combo_box(ui, &mut ty);

            self.in_ty.store(ty, Ordering::Release);
        }
    }

    fn show_disconnected(&self, ui: &mut eframe::egui::Ui, verbose: bool) {
        let ty = self.in_ty.load(Ordering::Relaxed);
        let mut samples = self.samples.load(Ordering::Acquire);
        let old_samples = samples;

        match ty {
            TimeUnit::Samples => {
                ui.add(egui::DragValue::new(&mut samples).clamp_range(1..=std::usize::MAX));
            }
            TimeUnit::Seconds => {
                let mut secs = samples as f32 / 44100.0;
                let input = PositiveInput::new(secs);
                input.show_disconnected(ui, verbose);

                secs = input.get_f32(&Value::None);
                samples = (secs * 44100.0).round() as _;
            }
            TimeUnit::Miliseconds => {
                let mut msecs = samples as f32 / 44100.0 * 1000.0;
                let input = PositiveInput::new(msecs);
                input.show_disconnected(ui, verbose);

                msecs = input.get_f32(&Value::None);
                samples = (msecs * 44100.0 / 1000.0).round() as _;
            }
        }

        self.samples
            .compare_exchange(old_samples, samples, Ordering::Release, Ordering::Relaxed)
            .ok();
    }
}
