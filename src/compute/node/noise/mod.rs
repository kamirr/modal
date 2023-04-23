use std::{
    any::Any,
    sync::{atomic::Ordering, Arc},
};

use rand::Rng;
use serde::{Deserialize, Serialize};

use crate::{
    serde_atomic_enum,
    util::{enum_combo_box, perlin::Perlin1D},
};

use super::{
    inputs::{freq::FreqInput, real::RealInput},
    Input, InputUi, Node, NodeConfig, NodeEvent, NodeList,
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
}

impl NoiseGenConfig {
    fn noise_type(&self) -> NoiseType {
        self.ty.load(Ordering::Relaxed)
    }
}

impl NodeConfig for NoiseGenConfig {
    fn show(&self, ui: &mut eframe::egui::Ui, _data: &dyn Any) {
        let mut ty = self.ty.load(Ordering::Acquire);

        enum_combo_box(ui, &mut ty);

        self.ty.store(ty, Ordering::Release);
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NoiseGen {
    config: Arc<NoiseGenConfig>,
    min: Arc<RealInput>,
    max: Arc<RealInput>,
    frequency_input: Arc<FreqInput>,
    ty: NoiseType,
    perlin_noise: Perlin1D,
    out: f32,
    t: u64,
}

#[typetag::serde]
impl Node for NoiseGen {
    fn feed(&mut self, data: &[Option<f32>]) -> Vec<NodeEvent> {
        let min = self.min.value(data[0]);
        let max = self.max.value(data[1]);
        let ty = self.config.noise_type();

        let emit = ty != self.ty;
        self.ty = ty;

        let m1_to_p1 = match ty {
            NoiseType::Uniform => rand::thread_rng().gen_range(min..=max),
            NoiseType::Perlin => {
                let frequency = self.frequency_input.value(data.get(2).copied().flatten());
                self.t += 1;
                let perlin_arg = self.t as f32 / 44100.0 * frequency;

                self.perlin_noise.noise(perlin_arg)
            }
        };

        let z_to_p1 = (m1_to_p1 + 1.0) / 2.0;

        self.out = z_to_p1 * (max - min) + min;

        if emit {
            vec![NodeEvent::RecalcInputs(self.inputs())]
        } else {
            Default::default()
        }
    }

    fn read(&self) -> f32 {
        self.out
    }

    fn config(&self) -> Option<Arc<dyn NodeConfig>> {
        Some(Arc::clone(&self.config) as Arc<_>)
    }

    fn inputs(&self) -> Vec<Input> {
        let mut ins = vec![
            Input::with_default("min", &self.min),
            Input::with_default("max", &self.max),
        ];

        match self.ty {
            NoiseType::Perlin => ins.push(Input::with_default("f", &self.frequency_input)),
            _ => {}
        }

        ins
    }
}

fn noise_gen() -> Box<dyn Node> {
    Box::new(NoiseGen {
        config: Arc::new(NoiseGenConfig {
            ty: AtomicNoiseType::new(NoiseType::Uniform),
        }),
        min: Arc::new(RealInput::new(-1.0)),
        max: Arc::new(RealInput::new(1.0)),
        frequency_input: Arc::new(FreqInput::new(440.0)),
        ty: NoiseType::Uniform,
        perlin_noise: Perlin1D::new(),
        out: 0.0,
        t: 0,
    })
}

pub struct Noise;

impl NodeList for Noise {
    fn all(&self) -> Vec<(Box<dyn Node>, String)> {
        vec![(noise_gen(), "Noise Generator".into())]
    }
}
