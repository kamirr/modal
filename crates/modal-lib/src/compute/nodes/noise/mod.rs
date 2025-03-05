use std::{
    any::Any,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};

use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha12Rng;
use serde::{Deserialize, Serialize};

use crate::{
    compute::inputs::gain::GainInput,
    serde_atomic_enum,
    util::{enum_combo_box, perlin::Perlin1D},
};
use runtime::{
    node::{Input, Node, NodeConfig, NodeEvent},
    ExternInputs, Value,
};

use super::NodeList;
use crate::compute::inputs::{
    freq::FreqInput,
    real::RealInput,
    trigger::{TriggerInput, TriggerMode},
};

#[atomic_enum::atomic_enum]
#[derive(Serialize, Deserialize, PartialEq, derive_more::Display, strum::EnumIter)]
enum NoiseType {
    Uniform,
    Perlin,
}

serde_atomic_enum!(AtomicNoiseType);

impl Eq for NoiseType {}

#[derive(Debug, Serialize, Deserialize)]
struct NoiseGenConfig {
    ty: AtomicNoiseType,
    manual_range: AtomicBool,
}

impl NoiseGenConfig {
    fn noise_type(&self) -> NoiseType {
        self.ty.load(Ordering::Relaxed)
    }

    fn manual_range(&self) -> bool {
        self.manual_range.load(Ordering::Relaxed)
    }
}

impl NodeConfig for NoiseGenConfig {
    fn show(&self, ui: &mut eframe::egui::Ui, _data: &dyn Any) {
        let mut manual_range = self.manual_range.load(Ordering::Relaxed);
        let mut ty = self.ty.load(Ordering::Relaxed);

        ui.checkbox(&mut manual_range, "Manual range");
        enum_combo_box(ui, &mut ty);

        self.manual_range.store(manual_range, Ordering::Relaxed);
        self.ty.store(ty, Ordering::Relaxed);
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NoiseGen {
    config: Arc<NoiseGenConfig>,
    latch: Arc<TriggerInput>,
    reset: Arc<TriggerInput>,
    gain: Arc<GainInput>,
    min: Arc<RealInput>,
    max: Arc<RealInput>,
    frequency_input: Arc<FreqInput>,

    manual_range: bool,
    ty: NoiseType,
    perlin_noise: Perlin1D,
    out: f32,
    t: u64,

    rng: ChaCha12Rng,
}

#[typetag::serde]
impl Node for NoiseGen {
    fn feed(&mut self, _inputs: &ExternInputs, data: &[Value]) -> Vec<NodeEvent> {
        // Latch continuously if disconnected
        let latch = if data[0].disconnected() {
            true
        } else {
            self.latch.trigger(&data[0])
        };

        let reset = self.reset.trigger(&data[1]);
        if reset {
            self.rng = ChaCha12Rng::from_seed([0xFE; 32]);
            self.t = 0;
        }

        let ty = self.config.noise_type();
        let manual_range = self.config.manual_range();

        let emit = ty != self.ty || manual_range != self.manual_range;
        self.ty = ty;
        self.manual_range = manual_range;

        let (min, max) = if self.manual_range && data.len() >= 4 {
            let min = self.min.get_f32(&data[2]);
            let max = self.max.get_f32(&data[3]);
            (min, max)
        } else {
            let gain = self.gain.get_multiplier(&data[2]);
            (-gain, gain)
        };

        let m1_to_p1 = match ty {
            NoiseType::Uniform => self.rng.gen_range(-1.0..=1.0),
            NoiseType::Perlin => {
                let frequency = self.frequency_input.get_f32(
                    data.get(if self.manual_range { 4 } else { 3 })
                        .unwrap_or(&Value::None),
                );
                self.t += 1;
                let perlin_arg = self.t as f32 / 44100.0 * frequency;

                self.perlin_noise.noise(perlin_arg)
            }
        };

        let z_to_p1 = (m1_to_p1 + 1.0) / 2.0;

        if latch {
            self.out = z_to_p1 * (max - min) + min;
        }

        if emit {
            vec![NodeEvent::RecalcInputs(self.inputs())]
        } else {
            Default::default()
        }
    }

    fn read(&self, out: &mut [Value]) {
        out[0] = Value::Float(self.out);
    }
    fn config(&self) -> Option<Arc<dyn NodeConfig>> {
        Some(Arc::clone(&self.config) as Arc<_>)
    }

    fn inputs(&self) -> Vec<Input> {
        let mut ins = vec![
            Input::stateful("latch", &self.latch),
            Input::stateful("reset", &self.reset),
        ];

        if self.manual_range {
            ins.push(Input::stateful("min", &self.min));
            ins.push(Input::stateful("max", &self.max));
        } else {
            ins.push(Input::stateful("gain", &self.gain));
        }

        if self.ty == NoiseType::Perlin {
            ins.push(Input::stateful("f", &self.frequency_input))
        }

        ins
    }
}

fn noise_gen() -> Box<dyn Node> {
    Box::new(NoiseGen {
        config: Arc::new(NoiseGenConfig {
            ty: AtomicNoiseType::new(NoiseType::Uniform),
            manual_range: AtomicBool::new(false),
        }),
        latch: Arc::new(TriggerInput::new(TriggerMode::Up, 0.5)),
        reset: Arc::new(TriggerInput::new(TriggerMode::Up, 0.5)),
        gain: Arc::new(GainInput::unit()),
        min: Arc::new(RealInput::new(-1.0)),
        max: Arc::new(RealInput::new(1.0)),
        frequency_input: Arc::new(FreqInput::new(440.0)),
        manual_range: false,
        ty: NoiseType::Uniform,
        perlin_noise: Perlin1D::new(),
        out: 0.0,
        t: 0,

        rng: ChaCha12Rng::from_seed([0xFE; 32]),
    })
}

pub struct Noise;

impl NodeList for Noise {
    fn all(&self) -> Vec<(Box<dyn Node>, String, Vec<String>)> {
        vec![(
            noise_gen(),
            "Noise".into(),
            vec!["Noise".into(), "Source".into()],
        )]
    }
}
