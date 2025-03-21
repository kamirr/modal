use std::{
    any::Any,
    sync::{atomic::Ordering, Arc},
};

use atomic_float::AtomicF32;
use eframe::egui::DragValue;
use serde::{Deserialize, Serialize};

use runtime::{
    node::{Input, Node, NodeConfig, NodeEvent},
    ExternInputs, Value, ValueKind,
};

use crate::compute::inputs::gate::{GateInput, GateInputState};

#[derive(Debug, Serialize, Deserialize)]
struct AdsrConfig {
    attack: AtomicF32,
    decay: AtomicF32,
    sustain_ratio: AtomicF32,
    release: AtomicF32,
}

impl NodeConfig for AdsrConfig {
    fn show(&self, ui: &mut eframe::egui::Ui, _data: &dyn Any) {
        let mut attack = self.attack.load(Ordering::Acquire);
        let mut decay = self.decay.load(Ordering::Acquire);
        let mut sustain_ratio = self.sustain_ratio.load(Ordering::Acquire) * 100.0;
        let mut release = self.release.load(Ordering::Acquire);

        ui.horizontal(|ui| {
            ui.label("attack");
            ui.add(DragValue::new(&mut attack).range(0.01..=1.0));
        });
        ui.horizontal(|ui| {
            ui.label("decay");
            ui.add(DragValue::new(&mut decay).range(0.01..=1.0));
        });
        ui.horizontal(|ui| {
            ui.label("sustain %");
            ui.add(DragValue::new(&mut sustain_ratio).range(0.0..=100.0));
        });
        ui.horizontal(|ui| {
            ui.label("release");
            ui.add(DragValue::new(&mut release).range(0.01..=1.0));
        });

        self.attack.store(attack, Ordering::Release);
        self.decay.store(decay, Ordering::Release);
        self.sustain_ratio
            .store(sustain_ratio / 100.0, Ordering::Release);
        self.release.store(release, Ordering::Release);
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
enum AdsrState {
    Attack,
    Decay,
    Sustain,
    Release,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Adsr {
    config: Arc<AdsrConfig>,
    gate: Arc<GateInput>,
    gate_state: GateInputState,
    state: AdsrState,
    attack_start_gain: f32,
    release_start_gain: f32,
    gain: f32,
    out: f32,
    cnt: usize,
}

impl Default for Adsr {
    fn default() -> Self {
        Self::new()
    }
}

impl Adsr {
    pub fn new() -> Self {
        Adsr {
            config: Arc::new(AdsrConfig {
                attack: AtomicF32::new(0.05),
                decay: AtomicF32::new(0.05),
                sustain_ratio: AtomicF32::new(0.7),
                release: AtomicF32::new(0.5),
            }),
            gate: Arc::new(GateInput::new(0.5)),
            gate_state: GateInputState::default(),
            state: AdsrState::Release,
            attack_start_gain: 0.0,
            release_start_gain: 0.0,
            gain: 0.0,
            out: 0.0,
            cnt: 0,
        }
    }
}

#[typetag::serde]
impl Node for Adsr {
    fn feed(&mut self, _inputs: &ExternInputs, data: &[Value]) -> Vec<NodeEvent> {
        let _ = self.gate.gate(&mut self.gate_state, &data[0]);
        let sig = data[1].as_float().unwrap_or(1.0);

        let conf_attack = self.config.attack.load(Ordering::Relaxed);
        let conf_decay = self.config.decay.load(Ordering::Relaxed);
        let conf_sustain_r = self.config.sustain_ratio.load(Ordering::Relaxed);
        let conf_release = self.config.release.load(Ordering::Relaxed);

        if self.gate.positive_edge() {
            self.state = AdsrState::Attack;
            self.attack_start_gain = self.gain;
            self.cnt = 0;
        } else if self.gate.negative_edge() {
            self.state = AdsrState::Release;
            self.release_start_gain = self.gain;
            self.cnt = 0;
        }

        let t = (self.cnt as f32) / 44100.0;

        match self.state {
            AdsrState::Attack => {
                if t >= conf_attack {
                    self.gain = 1.0;
                    self.state = AdsrState::Decay;
                    self.cnt = 0;
                } else {
                    self.gain =
                        (t / conf_attack) + self.attack_start_gain * (1.0 - t / conf_attack);
                }
            }
            AdsrState::Decay => {
                if t >= conf_decay {
                    self.gain = conf_sustain_r;
                    self.state = AdsrState::Sustain;
                    self.cnt = 0;
                } else {
                    let r = t / conf_decay;
                    self.gain = (conf_sustain_r - 1.0) * r + 1.0;
                }
            }
            AdsrState::Sustain => {
                self.gain = conf_sustain_r;
            }
            AdsrState::Release => {
                if t >= conf_release {
                    self.gain = 0.0;
                } else {
                    self.gain = self.release_start_gain * (1.0 - t / conf_release);
                }
            }
        }

        self.out = self.gain * sig;

        self.cnt += 1;

        Default::default()
    }

    fn read(&self, out: &mut [Value]) {
        out[0] = Value::Float(self.out)
    }

    fn config(&self) -> Option<Arc<dyn NodeConfig>> {
        Some(Arc::clone(&self.config) as Arc<_>)
    }

    fn inputs(&self) -> Vec<Input> {
        vec![
            Input::stateful("gate", &self.gate),
            Input::new("signal", ValueKind::Float),
        ]
    }
}

pub fn adsr() -> Box<dyn Node> {
    Box::new(Adsr::new())
}
