use std::sync::atomic::{AtomicUsize, Ordering};

use eframe::egui;
use serde::{Deserialize, Serialize};

use crate::{compute::node::InputUi, serde_atomic_enum};

#[atomic_enum::atomic_enum]
#[derive(PartialEq, Eq)]
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
    fn show(&self, ui: &mut eframe::egui::Ui) {
        let mut ty = self.in_ty.load(Ordering::Acquire);
        let mut samples = self.samples.load(Ordering::Acquire);

        ui.horizontal(|ui| {
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

            egui::ComboBox::from_label("")
                .selected_text(format!("{ty:?}"))
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut ty, TimeUnit::Samples, "Samples");
                    ui.selectable_value(&mut ty, TimeUnit::Seconds, "Seconds");
                });
        });

        self.in_ty.store(ty, Ordering::Release);
        self.samples.store(samples, Ordering::Release);
    }

    fn value(&self) -> f32 {
        self.samples.load(Ordering::Relaxed) as f32
    }
}
