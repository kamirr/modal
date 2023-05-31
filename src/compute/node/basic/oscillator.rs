use std::{
    any::Any,
    f32::consts::PI,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};

use serde::{Deserialize, Serialize};

use crate::{
    compute::{
        node::{
            inputs::{
                angle::AngleInput,
                beat::{BeatInput, BeatResponse},
                freq::FreqInput,
                real::RealInput,
                wave::WaveInput,
            },
            Input, Node, NodeConfig, NodeEvent,
        },
        Value,
    },
    wave::WaveScale,
};

#[derive(Debug, Serialize, Deserialize)]
struct OscillatorConfig {
    manual_range: AtomicBool,
    bpm_sync: AtomicBool,
}

impl NodeConfig for OscillatorConfig {
    fn show(&self, ui: &mut eframe::egui::Ui, _data: &dyn Any) {
        let mut manual_range = self.manual_range.load(Ordering::Acquire);
        let mut bpm_sync = self.bpm_sync.load(Ordering::Acquire);

        ui.checkbox(&mut manual_range, "Manual range");
        ui.checkbox(&mut bpm_sync, "BPM Sync");

        self.manual_range.store(manual_range, Ordering::Release);
        self.bpm_sync.store(bpm_sync, Ordering::Release);
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct Oscillator {
    config: Arc<OscillatorConfig>,
    freq: Arc<FreqInput>,
    beat: Arc<BeatInput>,
    phase: Arc<AngleInput>,
    wave: Arc<WaveInput>,
    min: Arc<RealInput>,
    max: Arc<RealInput>,
    t: f32,
    out: f32,

    manual_range: bool,
    bpm_sync: bool,
    hz: f32,
}

impl Oscillator {
    fn hz_to_dt() -> f32 {
        1.0 / 44100.0
    }
}

#[typetag::serde]
impl Node for Oscillator {
    fn feed(&mut self, data: &[Value]) -> Vec<NodeEvent> {
        if self.bpm_sync {
            if let Some(BeatResponse { period_secs }) = self.beat.process(&data[0]) {
                self.hz = 1.0 / period_secs;
                self.t = 0.0;
            }
        } else {
            self.hz = self.freq.get_f32(&data[0]);
        }

        let wave = self.wave.as_f32(&data[1]);

        let phase_0_2 = self.phase.radians(&data[2]) / PI;

        let min = self.min.get_f32(data.get(3).unwrap_or(&Value::Float(-1.0)));
        let max = self.max.get_f32(data.get(4).unwrap_or(&Value::Float(1.0)));

        let step = self.hz * Self::hz_to_dt() * 2.0;
        self.t = (self.t + step) % 2.0;

        let adjusted_t = (self.t + phase_0_2) % 2.0;

        self.out = (WaveScale::new(wave.clamp(0.0, 0.99)).sample(adjusted_t) / 2.0 + 0.5)
            * (max - min)
            + min;

        let manual_range = self.config.manual_range.load(Ordering::Relaxed);
        let bpm_sync = self.config.bpm_sync.load(Ordering::Relaxed);
        let emit_change = manual_range != self.manual_range || bpm_sync != self.bpm_sync;
        self.manual_range = manual_range;
        self.bpm_sync = bpm_sync;

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
        let mut inputs = Vec::new();

        if !self.bpm_sync {
            inputs.push(Input::stateful("f", &self.freq))
        } else {
            inputs.push(Input::stateful("beat", &self.beat))
        }

        inputs.push(Input::stateful("shape", &self.wave));
        inputs.push(Input::stateful("phase", &self.phase));

        if self.manual_range {
            inputs.extend([
                Input::stateful("min", &self.min),
                Input::stateful("max", &self.max),
            ])
        }

        inputs
    }
}

pub fn oscillator() -> Box<dyn Node> {
    Box::new(Oscillator {
        config: Arc::new(OscillatorConfig {
            manual_range: AtomicBool::new(false),
            bpm_sync: AtomicBool::new(false),
        }),
        freq: Arc::new(FreqInput::new(440.0)),
        beat: Arc::new(BeatInput::new(false)),
        wave: Arc::new(WaveInput::new(0.0)),
        phase: Arc::new(AngleInput::new(0.0)),
        min: Arc::new(RealInput::new(-1.0)),
        max: Arc::new(RealInput::new(1.0)),
        t: 0.0,
        out: 0.0,
        manual_range: false,
        bpm_sync: false,
        hz: 440.0,
    })
}
