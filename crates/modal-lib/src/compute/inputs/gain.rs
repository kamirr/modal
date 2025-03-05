use atomic_float::AtomicF32;
use eframe::egui::DragValue;
use serde::{Deserialize, Serialize};
use std::{fmt::Display, sync::atomic::Ordering};

use runtime::{node::InputUi, Value, ValueKind};

use crate::{serde_atomic_enum, util::enum_combo_box};

#[atomic_enum::atomic_enum]
#[derive(PartialEq, Eq, strum::EnumIter)]
enum GainKind {
    Db,
    Mul,
}

impl Display for GainKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                GainKind::Db => "dB",
                GainKind::Mul => "×",
            }
        )
    }
}

serde_atomic_enum!(AtomicGainKind);

#[derive(Debug, Serialize, Deserialize)]
pub struct GainInput {
    gain: AtomicF32,
    kind: AtomicGainKind,
    min: f32,
    max: f32,
}

impl GainInput {
    pub fn new(mult: f32) -> Self {
        GainInput {
            gain: AtomicF32::new(mult),
            kind: AtomicGainKind::new(GainKind::Db),
            min: 0.0,
            max: 4.0,
        }
    }

    pub fn unit() -> Self {
        Self::new(1.0)
    }

    pub fn min(mut self, min: f32) -> Self {
        assert!(min >= 0.0);
        self.min = min;
        self
    }

    pub fn max(mut self, max: f32) -> Self {
        self.max = max;
        self
    }

    pub fn get_multiplier(&self, recv: &Value) -> f32 {
        recv.as_float()
            .map(|gain_in| match self.kind.load(Ordering::Relaxed) {
                GainKind::Db => 10f32.powf(gain_in / 10.0),
                GainKind::Mul => gain_in,
            })
            .unwrap_or(self.gain.load(Ordering::Relaxed))
    }
}

impl InputUi for GainInput {
    fn value_kind(&self) -> ValueKind {
        ValueKind::Float
    }

    fn show_always(&self, ui: &mut eframe::egui::Ui, _verbose: bool) {
        let mut kind = self.kind.load(Ordering::Relaxed);
        enum_combo_box(ui, &mut kind);
        self.kind.store(kind, Ordering::Relaxed);
    }

    fn show_disconnected(&self, ui: &mut eframe::egui::Ui, _verbose: bool) {
        let mut mul_gain = self.gain.load(Ordering::Relaxed);
        let kind = self.kind.load(Ordering::Relaxed);

        match kind {
            GainKind::Db => {
                let mut db_gain = 10.0 * mul_gain.log10();
                let min = 10.0 * self.min.log10();
                let max = 10.0 * self.max.log10();

                // mul -> db -> mul roundtrip may introduce error, don't adjust
                // value if UI does not report any change.
                if ui
                    .add(DragValue::new(&mut db_gain).range(min..=max))
                    .changed()
                {
                    mul_gain = 10f32.powf(db_gain / 10.0);
                }
            }
            GainKind::Mul => {
                ui.add(DragValue::new(&mut mul_gain).range(self.min..=self.max));
            }
        }

        self.gain.store(mul_gain, Ordering::Relaxed);
    }
}
