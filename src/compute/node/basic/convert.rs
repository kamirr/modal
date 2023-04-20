use std::sync::{atomic::Ordering, Arc};

use eframe::egui;
use serde::{Deserialize, Serialize};

use crate::{
    compute::node::{Input, Node, NodeConfig, NodeEvent},
    serde_atomic_enum,
};

#[atomic_enum::atomic_enum]
#[derive(PartialEq, Eq)]
pub enum ConvTy {
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

        egui::ComboBox::from_label("")
            .selected_text(format!("{ty:?}"))
            .show_ui(ui, |ui| {
                ui.selectable_value(&mut ty, ConvTy::FreqToTime, "Freq to Time");
            });

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
    fn feed(&mut self, data: &[Option<f32>]) -> Vec<NodeEvent> {
        self.out = match self.conf.convert_type() {
            ConvTy::FreqToTime => 44100.0 / data[0].unwrap_or(0.0),
        };

        Default::default()
    }

    fn read(&self) -> f32 {
        self.out
    }

    fn config(&self) -> Option<Arc<dyn NodeConfig>> {
        Some(Arc::clone(&self.conf) as Arc<_>)
    }

    fn inputs(&self) -> Vec<Input> {
        vec![Input::new("in")]
    }
}

pub fn convert() -> Box<dyn Node> {
    Box::new(Convert::new(ConvTy::FreqToTime))
}
