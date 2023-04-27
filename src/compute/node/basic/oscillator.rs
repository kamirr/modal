use std::{
    any::Any,
    f32::consts::PI,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};

use atomic_enum::atomic_enum;
use serde::{Deserialize, Serialize};

use crate::{
    compute::{
        node::{
            inputs::{freq::FreqInput, real::RealInput},
            Input, Node, NodeConfig, NodeEvent,
        },
        Value,
    },
    serde_atomic_enum,
    util::enum_combo_box,
};

#[atomic_enum]
#[derive(PartialEq, Eq, Serialize, derive_more::Display, strum::EnumIter)]
enum OscType {
    Sine = 0,
    Triangle,
    Square,
    Saw,
}

serde_atomic_enum!(AtomicOscType);

#[derive(Debug, Serialize, Deserialize)]
struct OscillatorConfig {
    ty: AtomicOscType,
    manual_range: AtomicBool,
}

impl NodeConfig for OscillatorConfig {
    fn show(&self, ui: &mut eframe::egui::Ui, _data: &dyn Any) {
        let mut ty = self.ty.load(Ordering::Acquire);
        let mut manual_range = self.manual_range.load(Ordering::Acquire);

        enum_combo_box(ui, &mut ty);

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
    fn feed(&mut self, data: &[Value]) -> Vec<NodeEvent> {
        let f = self.f.get_f32(&data[0]);
        let min = self.min.get_f32(data.get(1).unwrap_or(&Value::Float(-1.0)));
        let max = self.max.get_f32(data.get(2).unwrap_or(&Value::Float(1.0)));

        let step = f * Self::hz_to_dt();
        self.t = (self.t + step) % 4.0;

        let m1_to_p1 = match self.config.ty.load(Ordering::Relaxed) {
            OscType::Sine => (self.t * 2.0 * PI).sin(),
            OscType::Triangle => 4.0 * (self.t - (self.t + 0.5).floor()).abs() - 1.0,
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

    fn read(&self) -> Value {
        Value::Float(self.out)
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
