use std::{
    any::Any,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};

use serde::{Deserialize, Serialize};

use crate::{
    compute::{
        node::{
            inputs::{freq::FreqInput, real::RealInput, wave::WaveInput},
            Input, Node, NodeConfig, NodeEvent,
        },
        Value, ValueKind,
    },
    wave::WaveScale,
};

#[derive(Debug, Serialize, Deserialize)]
struct OscillatorConfig {
    manual_range: AtomicBool,
}

impl NodeConfig for OscillatorConfig {
    fn show(&self, ui: &mut eframe::egui::Ui, _data: &dyn Any) {
        let mut manual_range = self.manual_range.load(Ordering::Acquire);

        ui.checkbox(&mut manual_range, "Manual range");

        self.manual_range.store(manual_range, Ordering::Release);
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct Oscillator {
    config: Arc<OscillatorConfig>,
    wave: Arc<WaveInput>,
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
        let wave = self.wave.as_f32(&data[1]);
        let min = self.min.get_f32(data.get(2).unwrap_or(&Value::Float(-1.0)));
        let max = self.max.get_f32(data.get(3).unwrap_or(&Value::Float(1.0)));

        let step = f * Self::hz_to_dt() * 2.0;
        self.t = (self.t + step) % 2.0;

        self.out =
            (WaveScale::new(wave.clamp(0.0, 0.99)).sample(self.t) / 2.0 + 0.5) * (max - min) + min;

        let manual_range = self.config.manual_range.load(Ordering::Relaxed);
        let emit_change = manual_range != self.manual_range;
        self.manual_range = manual_range;

        if emit_change {
            vec![NodeEvent::RecalcInputs(self.inputs())]
        } else {
            Default::default()
        }
    }

    fn read(&self, out: &mut [Value]) {
        out[0] = Value::Float(self.out)
    }

    fn config(&self) -> Option<Arc<dyn NodeConfig>> {
        Some(Arc::clone(&self.config) as Arc<dyn NodeConfig>)
    }

    fn inputs(&self) -> Vec<Input> {
        let mut inputs = vec![
            Input::with_default("f", ValueKind::Float, &self.f),
            Input::with_default("shape", ValueKind::Float, &self.wave),
        ];
        if self.manual_range {
            inputs.extend([
                Input::with_default("min", ValueKind::Float, &self.min),
                Input::with_default("max", ValueKind::Float, &self.max),
            ])
        }

        inputs
    }
}

pub fn oscillator() -> Box<dyn Node> {
    Box::new(Oscillator {
        config: Arc::new(OscillatorConfig {
            manual_range: AtomicBool::new(false),
        }),
        wave: Arc::new(WaveInput::new(0.0)),
        f: Arc::new(FreqInput::new(440.0)),
        min: Arc::new(RealInput::new(-1.0)),
        max: Arc::new(RealInput::new(1.0)),
        t: 0.0,
        out: 0.0,
        manual_range: false,
    })
}
