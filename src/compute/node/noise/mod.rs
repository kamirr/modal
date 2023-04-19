use std::{
    any::Any,
    sync::{Arc, RwLock},
};

use eframe::egui::{ComboBox, DragValue};
use rand::Rng;
use rand_distr::{Distribution, Normal};
use serde::{Deserialize, Serialize};

use super::{inputs::real::RealInput, Input, InputUi, Node, NodeConfig, NodeEvent, NodeList};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
enum NoiseType {
    Uniform,
    White { std_dev: f32 },
}

impl Eq for NoiseType {}

#[derive(Debug, Serialize, Deserialize)]
struct NoiseGenConfig {
    #[serde(with = "crate::util::serde_rwlock")]
    ty: RwLock<NoiseType>,
}

impl NodeConfig for NoiseGenConfig {
    fn show(&self, ui: &mut eframe::egui::Ui, _data: &dyn Any) {
        let mut ty = self.ty.read().unwrap().clone();
        let old_ty = ty.clone();

        ComboBox::from_label("")
            .selected_text(format!("{ty:?}"))
            .show_ui(ui, |ui| {
                ui.selectable_value(&mut ty, NoiseType::Uniform, "Uniform");
                ui.selectable_value(&mut ty, NoiseType::White { std_dev: 0.1 }, "Normal");
            });

        if let NoiseType::White { std_dev } = &mut ty {
            ui.horizontal(|ui| {
                ui.label("σ");
                ui.add(DragValue::new(std_dev).clamp_range(0.0..=1.0).speed(0.02));
            });
        }

        if old_ty != ty {
            *self.ty.write().unwrap() = ty;
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NoiseGen {
    config: Arc<NoiseGenConfig>,
    min: Arc<RealInput>,
    max: Arc<RealInput>,
    out: f32,
}

#[typetag::serde]
impl Node for NoiseGen {
    fn feed(&mut self, data: &[Option<f32>]) -> Vec<NodeEvent> {
        let min = data[0].unwrap_or(self.min.value());
        let max = data[1].unwrap_or(self.max.value());
        let ty = self.config.ty.read().unwrap().clone();

        let m1_to_p1 = match ty {
            NoiseType::Uniform => rand::thread_rng().gen_range(min..=max),
            NoiseType::White { std_dev } => {
                let normal = Normal::new(0.0, std_dev).unwrap();
                normal.sample(&mut rand::thread_rng())
            }
        };

        let z_to_p1 = (m1_to_p1 + 1.0) / 2.0;

        self.out = z_to_p1 * (max - min) + min;

        Default::default()
    }

    fn read(&self) -> f32 {
        self.out
    }

    fn config(&self) -> Option<Arc<dyn NodeConfig>> {
        Some(Arc::clone(&self.config) as Arc<_>)
    }

    fn inputs(&self) -> Vec<Input> {
        vec![
            Input::with_default("min", &self.min),
            Input::with_default("max", &self.max),
        ]
    }
}

fn noise_gen() -> Box<dyn Node> {
    Box::new(NoiseGen {
        config: Arc::new(NoiseGenConfig {
            ty: RwLock::new(NoiseType::Uniform),
        }),
        min: Arc::new(RealInput::new(-1.0)),
        max: Arc::new(RealInput::new(1.0)),
        out: 0.0,
    })
}

pub struct Noise;

impl NodeList for Noise {
    fn all(&self) -> Vec<(Box<dyn Node>, String)> {
        vec![(noise_gen(), "Noise Generator".into())]
    }
}
