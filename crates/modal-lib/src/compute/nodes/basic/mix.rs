use std::{
    any::Any,
    sync::{
        atomic::{AtomicU32, Ordering},
        Arc,
    },
};

use eframe::egui::DragValue;
use serde::{Deserialize, Serialize};

use runtime::{
    node::{Input, Node, NodeConfig, NodeEvent},
    ExternInputs, Value,
};

use crate::compute::inputs::slider::SliderInput;

#[derive(Debug, Serialize, Deserialize)]
struct MixConfig {
    new_ins: AtomicU32,
    ins: AtomicU32,
}

impl NodeConfig for MixConfig {
    fn show(&self, ui: &mut eframe::egui::Ui, _data: &dyn Any) {
        let mut ins = self.ins.load(Ordering::Acquire);

        ui.horizontal(|ui| {
            ui.label("inputs");

            if ui
                .add(DragValue::new(&mut ins).range(0..=std::u32::MAX))
                .lost_focus()
            {
                self.ins.store(ins, Ordering::Release);
            }
        });

        self.new_ins.store(ins, Ordering::Release);
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Mix {
    config: Arc<MixConfig>,
    weights: Vec<Arc<SliderInput>>,
    ins: u32,
    out: f32,
}

impl Mix {
    pub fn new(ins: u32) -> Self {
        Mix {
            config: Arc::new(MixConfig {
                new_ins: AtomicU32::new(ins),
                ins: AtomicU32::new(ins),
            }),
            weights: (0..ins)
                .map(|_| Arc::new(SliderInput::new(1.0, 0.0, 1.0).show_connected(true)))
                .collect(),
            ins,
            out: 0.0,
        }
    }
}

#[typetag::serde]
impl Node for Mix {
    fn feed(&mut self, _inputs: &ExternInputs, data: &[Value]) -> Vec<NodeEvent> {
        self.out = data
            .iter()
            .zip(self.weights.iter())
            .map(|(sample, weight)| {
                sample.as_float().unwrap_or(0.0) * weight.as_f32(&Value::Disconnected)
            })
            .sum::<f32>()
            / self.weights.len() as f32;

        let new_ins = self.config.ins.load(Ordering::Relaxed);
        let emit_ev = new_ins != self.ins;
        self.ins = new_ins;

        if self.ins as usize != self.weights.len() {
            self.weights.resize_with(self.ins as usize, || {
                Arc::new(SliderInput::new(1.0, 0.0, 1.0).show_connected(true))
            });
        }

        if emit_ev {
            vec![NodeEvent::RecalcInputs(self.inputs())]
        } else {
            Default::default()
        }
    }

    fn read(&self, out: &mut [Value]) {
        out[0] = Value::Float(self.out)
    }

    fn config(&self) -> Option<Arc<dyn NodeConfig>> {
        Some(Arc::clone(&self.config) as Arc<_>)
    }

    fn inputs(&self) -> Vec<Input> {
        (0..self.ins)
            .map(|i| Input::stateful(format!("sig {i}"), &self.weights[i as usize]))
            .collect()
    }
}

pub fn mix() -> Box<dyn Node> {
    Box::new(Mix::new(2))
}
