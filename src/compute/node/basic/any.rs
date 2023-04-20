use std::sync::{
    atomic::{AtomicU32, Ordering},
    Arc,
};

use eframe::egui::DragValue;
use serde::{Deserialize, Serialize};

use crate::compute::node::{
    inputs::trigger::{TriggerInput, TriggerMode},
    Input, InputUi, Node, NodeConfig, NodeEvent,
};

#[derive(Debug, Serialize, Deserialize)]
struct AnyConfig {
    new_ins: AtomicU32,
    ins: AtomicU32,
}

impl NodeConfig for AnyConfig {
    fn show(&self, ui: &mut eframe::egui::Ui, _data: &dyn std::any::Any) {
        let mut ins = self.ins.load(Ordering::Acquire);

        ui.horizontal(|ui| {
            ui.label("inputs");

            if ui
                .add(DragValue::new(&mut ins).clamp_range(0..=std::u32::MAX))
                .lost_focus()
            {
                self.ins.store(ins, Ordering::Release);
            }
        });

        self.new_ins.store(ins, Ordering::Release);
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Any {
    config: Arc<AnyConfig>,
    defaults: Vec<Arc<TriggerInput>>,
    ins: u32,
    out: f32,
}

impl Any {
    pub fn new(ins: u32) -> Self {
        Any {
            config: Arc::new(AnyConfig {
                new_ins: AtomicU32::new(ins),
                ins: AtomicU32::new(ins),
            }),
            defaults: (0..ins)
                .map(|_| Arc::new(TriggerInput::new(TriggerMode::Change, 0.5)))
                .collect(),
            ins,
            out: 0.0,
        }
    }
}

#[typetag::serde]
impl Node for Any {
    fn feed(&mut self, data: &[Option<f32>]) -> Vec<NodeEvent> {
        let emit = data
            .iter()
            .zip(self.defaults.iter())
            .map(|(sample, default)| default.value(*sample))
            .any(|f| f > 0.5);
        self.out = if emit { 1.0 } else { 0.0 };

        let new_ins = self.config.ins.load(Ordering::Relaxed);
        let emit_ev = new_ins != self.ins;
        self.ins = new_ins;

        if self.ins as usize != self.defaults.len() {
            self.defaults.resize_with(self.ins as usize, || {
                Arc::new(TriggerInput::new(TriggerMode::Up, 0.5))
            });
        }

        if emit_ev {
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
        (0..self.ins)
            .map(|i| Input::with_default(format!("sig {i}"), &self.defaults[i as usize]))
            .collect()
    }
}

pub fn any() -> Box<dyn Node> {
    Box::new(Any::new(2))
}
