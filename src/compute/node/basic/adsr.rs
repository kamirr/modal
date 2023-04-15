use std::sync::{atomic::Ordering, Arc};

use atomic_float::AtomicF32;
use eframe::egui::DragValue;
use serde::{Deserialize, Serialize};

use crate::compute::node::{Input, Node, NodeConfig, NodeEvent};

#[derive(Debug, Serialize, Deserialize)]
struct AdsrConfig {
    trigger: AtomicF32,
    attack: AtomicF32,
    decay: AtomicF32,
    sustain_ratio: AtomicF32,
    release: AtomicF32,
}

impl NodeConfig for AdsrConfig {
    fn show(&self, ui: &mut eframe::egui::Ui) {
        let mut trigger = self.trigger.load(Ordering::Acquire);
        let mut attack = self.attack.load(Ordering::Acquire);
        let mut decay = self.decay.load(Ordering::Acquire);
        let mut sustain_ratio = self.sustain_ratio.load(Ordering::Acquire) * 100.0;
        let mut release = self.release.load(Ordering::Acquire);

        ui.horizontal(|ui| {
            ui.label("trigger");
            ui.add(DragValue::new(&mut trigger).clamp_range(-1.0..=1.0));
        });
        ui.horizontal(|ui| {
            ui.label("attack");
            ui.add(DragValue::new(&mut attack).clamp_range(0.01..=1.0));
        });
        ui.horizontal(|ui| {
            ui.label("decay");
            ui.add(DragValue::new(&mut decay).clamp_range(0.01..=1.0));
        });
        ui.horizontal(|ui| {
            ui.label("sustain %");
            ui.add(DragValue::new(&mut sustain_ratio).clamp_range(0.0..=100.0));
        });
        ui.horizontal(|ui| {
            ui.label("release");
            ui.add(DragValue::new(&mut release).clamp_range(0.01..=1.0));
        });

        self.trigger.store(trigger, Ordering::Release);
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
    state: AdsrState,
    prev_trig: f32,
    out: f32,
    cnt: usize,
}

impl Adsr {
    pub fn new() -> Self {
        Adsr {
            config: Arc::new(AdsrConfig {
                trigger: AtomicF32::new(0.0),
                attack: AtomicF32::new(0.05),
                decay: AtomicF32::new(0.05),
                sustain_ratio: AtomicF32::new(0.7),
                release: AtomicF32::new(0.5),
            }),
            state: AdsrState::Release,
            prev_trig: 0.0,
            out: 0.0,
            cnt: 0,
        }
    }
}

#[typetag::serde]
impl Node for Adsr {
    fn feed(&mut self, data: &[Option<f32>]) -> Vec<NodeEvent> {
        let trigger = data[0].unwrap_or(0.0);
        let sig = data[1].unwrap_or(0.0);

        let conf_trigger = self.config.trigger.load(Ordering::Relaxed);
        let conf_attack = self.config.attack.load(Ordering::Relaxed);
        let conf_decay = self.config.decay.load(Ordering::Relaxed);
        let conf_sustain_r = self.config.sustain_ratio.load(Ordering::Relaxed);
        let conf_release = self.config.release.load(Ordering::Relaxed);

        if self.prev_trig < conf_trigger && trigger > conf_trigger {
            self.state = AdsrState::Attack;
            self.cnt = 0;
        }
        if trigger < conf_trigger && self.state != AdsrState::Release {
            self.state = AdsrState::Release;
            self.cnt = 0;
        }

        let t = (self.cnt as f32) / 44100.0;

        match self.state {
            AdsrState::Attack => {
                if t >= conf_attack {
                    self.out = sig;
                    self.state = AdsrState::Decay;
                    self.cnt = 0;
                } else {
                    self.out = sig * (t / conf_attack);
                }
            }
            AdsrState::Decay => {
                if t >= conf_decay {
                    self.out = sig * conf_sustain_r;
                    self.state = AdsrState::Sustain;
                    self.cnt = 0;
                } else {
                    let r = t / conf_decay;
                    self.out = sig * ((conf_sustain_r - 1.0) * r + 1.0);
                }
            }
            AdsrState::Sustain => {
                self.out = sig * conf_sustain_r;
            }
            AdsrState::Release => {
                if t >= conf_release {
                    self.out = 0.0;
                } else {
                    self.out = sig * conf_sustain_r * (1.0 - t / conf_release);
                }
            }
        }

        self.prev_trig = trigger;
        self.cnt += 1;

        Default::default()
    }

    fn read(&self) -> f32 {
        self.out
    }

    fn config(&self) -> Option<Arc<dyn NodeConfig>> {
        Some(Arc::clone(&self.config) as Arc<_>)
    }

    fn inputs(&self) -> Vec<Input> {
        vec![Input::new("trigger"), Input::new("signal")]
    }
}

pub fn adsr() -> Box<dyn Node> {
    Box::new(Adsr::new())
}
