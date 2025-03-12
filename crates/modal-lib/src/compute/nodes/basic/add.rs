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

use crate::compute::inputs::real::RealInput;

#[derive(Debug, Serialize, Deserialize)]
struct AddConfig {
    new_ins: AtomicU32,
    ins: AtomicU32,
}

impl NodeConfig for AddConfig {
    fn show(&self, ui: &mut eframe::egui::Ui, _data: &dyn Any) {
        let mut ins = self.ins.load(Ordering::Acquire);

        ui.horizontal(|ui| {
            ui.label("inputs");

            let response = ui.add(DragValue::new(&mut ins).range(0..=u32::MAX));

            if response.changed() {
                self.ins.store(ins, Ordering::Release);
            }

            if response.lost_focus() {
                self.new_ins.store(ins, Ordering::Release);
            }
        });
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Add {
    config: Arc<AddConfig>,
    defaults: Vec<Arc<RealInput>>,
    ins: u32,
    out: f32,
}

impl Add {
    pub fn new(ins: u32) -> Self {
        Add {
            config: Arc::new(AddConfig {
                new_ins: AtomicU32::new(ins),
                ins: AtomicU32::new(ins),
            }),
            defaults: (0..ins).map(|_| Arc::new(RealInput::new(0.0))).collect(),
            ins,
            out: 0.0,
        }
    }
}

#[typetag::serde]
impl Node for Add {
    fn feed(&mut self, _inputs: &ExternInputs, data: &[Value]) -> Vec<NodeEvent> {
        self.out = data
            .iter()
            .zip(self.defaults.iter())
            .map(|(sample, default)| default.get_f32(sample))
            .sum();

        let new_ins = self.config.new_ins.load(Ordering::Relaxed);
        let emit_ev = new_ins != self.ins;
        self.ins = new_ins;

        if self.ins as usize != self.defaults.len() {
            self.defaults
                .resize_with(self.ins as usize, || Arc::new(RealInput::new(0.0)));
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
            .map(|i| Input::stateful(format!("sig {i}"), &self.defaults[i as usize]))
            .collect()
    }
}

pub fn add() -> Box<dyn Node> {
    Box::new(Add::new(2))
}
