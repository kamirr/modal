use std::sync::{atomic::Ordering, Arc};

use crate::{serde_atomic_enum, util::enum_combo_box};
use runtime::{
    node::{Input, Node, NodeConfig, NodeEvent},
    Value, ValueKind,
};
use serde::{Deserialize, Serialize};

#[atomic_enum::atomic_enum]
#[derive(PartialEq, Eq, derive_more::Display, strum::EnumIter)]
pub enum ConvTy {
    #[display(fmt = "Freq to Time")]
    FreqToTime,
}

serde_atomic_enum!(AtomicConvTy);

#[derive(Debug, Serialize, Deserialize)]
struct ConvertConfig {
    ty: AtomicConvTy,
}

impl ConvertConfig {
    fn new(ty: ConvTy) -> Self {
        ConvertConfig {
            ty: AtomicConvTy::new(ty),
        }
    }

    fn convert_type(&self) -> ConvTy {
        self.ty.load(Ordering::Relaxed)
    }
}

impl NodeConfig for ConvertConfig {
    fn show(&self, ui: &mut eframe::egui::Ui, _data: &dyn std::any::Any) {
        let mut ty = self.ty.load(Ordering::Acquire);

        enum_combo_box(ui, &mut ty);

        self.ty.store(ty, Ordering::Release);
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Convert {
    conf: Arc<ConvertConfig>,
    out: f32,
}

impl Convert {
    fn new(ty: ConvTy) -> Self {
        Convert {
            conf: Arc::new(ConvertConfig::new(ty)),
            out: 0.0,
        }
    }
}

#[typetag::serde]
impl Node for Convert {
    fn feed(&mut self, data: &[Value]) -> Vec<NodeEvent> {
        self.out = match self.conf.convert_type() {
            ConvTy::FreqToTime => data[0].as_float().map(|f| 44100.0 / f).unwrap_or(0.0),
        };

        Default::default()
    }

    fn read(&self, out: &mut [Value]) {
        out[0] = Value::Float(self.out)
    }

    fn config(&self) -> Option<Arc<dyn NodeConfig>> {
        Some(Arc::clone(&self.conf) as Arc<_>)
    }

    fn inputs(&self) -> Vec<Input> {
        vec![Input::new("in", ValueKind::Float)]
    }
}

pub fn convert() -> Box<dyn Node> {
    Box::new(Convert::new(ConvTy::FreqToTime))
}
