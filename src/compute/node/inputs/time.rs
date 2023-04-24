use std::sync::atomic::{AtomicUsize, Ordering};

use eframe::egui;
use serde::{Deserialize, Serialize};

use crate::{compute::node::InputUi, serde_atomic_enum, util::enum_combo_box};

#[atomic_enum::atomic_enum]
#[derive(PartialEq, Eq, derive_more::Display, strum::EnumIter)]
enum TimeUnit {
    Samples,
    Seconds,
}

serde_atomic_enum!(AtomicTimeUnit);

#[derive(Debug, Serialize, Deserialize)]
pub struct TimeInput {
    samples: AtomicUsize,
    in_ty: AtomicTimeUnit,
}

impl TimeInput {
    pub fn new(samples: usize) -> Self {
        TimeInput {
            samples: AtomicUsize::new(samples),
            in_ty: AtomicTimeUnit::new(TimeUnit::Samples),
        }
    }
}

impl InputUi for TimeInput {
    fn show_always(&self, ui: &mut egui::Ui, verbose: bool) {
        if verbose {
            let mut ty = self.in_ty.load(Ordering::Acquire);

            enum_combo_box(ui, &mut ty);

            self.in_ty.store(ty, Ordering::Release);
        }
    }

    fn show_disconnected(&self, ui: &mut eframe::egui::Ui, _verbose: bool) {
        let ty = self.in_ty.load(Ordering::Relaxed);
        let mut samples = self.samples.load(Ordering::Acquire);

        match ty {
            TimeUnit::Samples => {
                ui.add(egui::DragValue::new(&mut samples).clamp_range(1..=std::usize::MAX));
            }
            TimeUnit::Seconds => {
                let mut secs = samples as f32 / 44100.0;
                ui.add(egui::DragValue::new(&mut secs).clamp_range(0.0001..=std::f32::MAX));
                samples = (secs * 44100.0).round() as _;
            }
        }

        self.samples.store(samples, Ordering::Release);
    }

    fn value(&self, recv: Option<f32>) -> f32 {
        recv.unwrap_or(self.samples.load(Ordering::Relaxed) as f32)
    }
}
