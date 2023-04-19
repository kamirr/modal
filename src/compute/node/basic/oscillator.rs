use std::{
    any::Any,
    f32::consts::PI,
    sync::{
        atomic::{AtomicBool, AtomicUsize, Ordering},
        Arc,
    },
};

use atomic_enum::atomic_enum;
use eframe::egui::ComboBox;
use serde::{Deserialize, Serialize};

use crate::compute::node::{
    inputs::{freq::FreqInput, real::RealInput},
    Input, InputUi, Node, NodeConfig, NodeEvent,
};

#[atomic_enum]
#[derive(PartialEq, Eq, Serialize)]
enum OscType {
    Sine = 0,
    Square,
    Saw,
}

impl Serialize for AtomicOscType {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.0.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for AtomicOscType {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        AtomicUsize::deserialize(deserializer).map(|inner| AtomicOscType(inner))
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct OscillatorConfig {
    ty: AtomicOscType,
    manual_range: AtomicBool,
}

impl NodeConfig for OscillatorConfig {
    fn show(&self, ui: &mut eframe::egui::Ui, _data: &dyn Any) {
        let mut ty = self.ty.load(Ordering::Acquire);
        let mut manual_range = self.manual_range.load(Ordering::Acquire);

        ComboBox::from_label("")
            .selected_text(format!("{ty:?}"))
            .show_ui(ui, |ui| {
                ui.selectable_value(&mut ty, OscType::Sine, "Sine");
                ui.selectable_value(&mut ty, OscType::Square, "Square");
                ui.selectable_value(&mut ty, OscType::Saw, "Saw");
            });
        ui.checkbox(&mut manual_range, "Manual range");

        self.ty.store(ty, Ordering::Release);
        self.manual_range.store(manual_range, Ordering::Release);
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct Oscillator {
    config: Arc<OscillatorConfig>,
    f: Arc<FreqInput>,
    min: Arc<RealInput>,
    max: Arc<RealInput>,
    t: f32,
    out: f32,
    manual_range: bool,
}

impl Oscillator {
    fn hz_to_dt() -> f32 {
        1.0 / 44100.0
    }
}

#[typetag::serde]
impl Node for Oscillator {
    fn feed(&mut self, data: &[Option<f32>]) -> Vec<NodeEvent> {
        let f = data[0].unwrap_or(self.f.value());
        let min = data
            .get(1)
            .unwrap_or(&Some(-1.0))
            .unwrap_or(self.min.value());
        let max = data
            .get(2)
            .unwrap_or(&Some(1.0))
            .unwrap_or(self.max.value());

        let step = f * Self::hz_to_dt();
        self.t = (self.t + step) % 4.0;

        let m1_to_p1 = match self.config.ty.load(Ordering::Relaxed) {
            OscType::Sine => (self.t * 2.0 * PI).sin(),
            OscType::Square => {
                if (self.t * 2.0 * PI).sin() > 0.0 {
                    1.0
                } else {
                    -1.0
                }
            }
            OscType::Saw => (self.t - self.t.floor()) * 2.0 - 1.0,
        };

        let zero_to_one = (m1_to_p1 + 1.0) / 2.0;
        self.out = zero_to_one * (max - min) + min;

        let manual_range = self.config.manual_range.load(Ordering::Relaxed);
        let emit_change = manual_range != self.manual_range;
        self.manual_range = manual_range;

        if emit_change {
            vec![NodeEvent::RecalcInputs(self.inputs())]
        } else {
            Default::default()
        }
    }

    fn read(&self) -> f32 {
        self.out
    }

    fn config(&self) -> Option<Arc<dyn NodeConfig>> {
        Some(Arc::clone(&self.config) as Arc<dyn NodeConfig>)
    }

    fn inputs(&self) -> Vec<Input> {
        let mut inputs = vec![Input::with_default("f", &self.f)];
        if self.manual_range {
            inputs.extend([
                Input::with_default("min", &self.min),
                Input::with_default("max", &self.max),
            ])
        }

        inputs
    }
}

pub fn oscillator() -> Box<dyn Node> {
    Box::new(Oscillator {
        config: Arc::new(OscillatorConfig {
            ty: AtomicOscType::new(OscType::Sine),
            manual_range: AtomicBool::new(false),
        }),
        f: Arc::new(FreqInput::new(440.0)),
        min: Arc::new(RealInput::new(-1.0)),
        max: Arc::new(RealInput::new(1.0)),
        t: 0.0,
        out: 0.0,
        manual_range: false,
    })
}
