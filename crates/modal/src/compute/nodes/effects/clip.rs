use std::sync::{atomic::Ordering, Arc};

use serde::{Deserialize, Serialize};

use crate::compute::inputs::slider::SliderInput;
use crate::{serde_atomic_enum, util::enum_combo_box};
use runtime::{
    node::{Input, Node, NodeConfig, NodeEvent},
    Value, ValueKind,
};

#[atomic_enum::atomic_enum]
#[derive(PartialEq, Eq, derive_more::Display, strum::EnumIter)]
pub enum ClipType {
    Hard,
    Poly,
    Tanh,
}

serde_atomic_enum!(AtomicClipType);

#[derive(Debug, Serialize, Deserialize)]
struct ClipConfig {
    ty: AtomicClipType,
}

impl ClipConfig {
    fn new(ty: ClipType) -> Self {
        ClipConfig {
            ty: AtomicClipType::new(ty),
        }
    }

    fn clip_ty(&self) -> ClipType {
        self.ty.load(Ordering::Relaxed)
    }
}

impl NodeConfig for ClipConfig {
    fn show(&self, ui: &mut eframe::egui::Ui, _data: &dyn std::any::Any) {
        let mut ty = self.ty.load(Ordering::Acquire);

        enum_combo_box(ui, &mut ty);

        self.ty.store(ty, Ordering::Release);
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Clip {
    config: Arc<ClipConfig>,
    level: Arc<SliderInput>,
    offset: Arc<SliderInput>,
    out: f32,
}

#[typetag::serde]
impl Node for Clip {
    fn config(&self) -> Option<Arc<dyn NodeConfig>> {
        Some(Arc::clone(&self.config) as Arc<_>)
    }

    fn feed(&mut self, data: &[Value]) -> Vec<NodeEvent> {
        let value = data[0].as_float().unwrap_or(0.0);
        let level = self.level.as_f32(&data[1]).max(0.0);
        let offset = self.offset.as_f32(&data[2]);

        let clip_ty = self.config.clip_ty();

        self.out = match clip_ty {
            ClipType::Hard => (value + offset).clamp(-level, level),
            ClipType::Poly => {
                let mut scaled = (value + offset) / level;

                scaled = if scaled <= -1.0 {
                    -1.0
                } else if scaled <= 1.0 {
                    1.5 * (scaled - scaled.powi(3) / 3.0)
                } else {
                    1.0
                };

                scaled * level - offset
            }
            ClipType::Tanh => {
                let mut scaled = (value + offset) / level;

                scaled = scaled.tanh();

                scaled * level - offset
            }
        };

        Default::default()
    }

    fn read(&self, out: &mut [Value]) {
        out[0] = Value::Float(self.out)
    }

    fn inputs(&self) -> Vec<Input> {
        vec![
            Input::new("value", ValueKind::Float),
            Input::stateful("level", &self.level),
            Input::stateful("offset", &self.offset),
        ]
    }
}

pub fn clip() -> Box<dyn Node> {
    Box::new(Clip {
        config: Arc::new(ClipConfig::new(ClipType::Hard)),
        level: Arc::new(SliderInput::new(1.0, 0.0, 1.0)),
        offset: Arc::new(SliderInput::new(0.0, -0.1, 0.1)),
        out: 0.0,
    })
}
