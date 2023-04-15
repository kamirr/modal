use std::{
    collections::VecDeque,
    sync::{
        atomic::{AtomicU8, AtomicUsize, Ordering},
        Arc,
    },
};

use eframe::egui::{ComboBox, DragValue};
use serde::Serialize;

use crate::compute::node::{Input, Node, NodeConfig, NodeEvent};

#[derive(Debug, Serialize)]
struct DelayConfig {
    samples: AtomicUsize,
    in_ty: AtomicU8,
}

impl NodeConfig for DelayConfig {
    fn show(&self, ui: &mut eframe::egui::Ui) {
        let mut in_ty = self.in_ty.load(Ordering::Acquire);
        let mut samples = self.samples.load(Ordering::Acquire);

        ComboBox::from_label("")
            .selected_text(if in_ty == 0 { "Samples" } else { "Seconds" })
            .show_ui(ui, |ui| {
                ui.selectable_value(&mut in_ty, 0, "Samples");
                ui.selectable_value(&mut in_ty, 1, "Seconds");
            });

        if in_ty == 0 {
            ui.add(DragValue::new(&mut samples));
        } else {
            let mut secs = samples as f32 / 44100.0;
            ui.add(DragValue::new(&mut secs));
            samples = (secs * 44100.0).round() as _;
        }

        self.in_ty.store(in_ty, Ordering::Release);
        self.samples.store(samples, Ordering::Release);
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct Delay {
    config: Arc<DelayConfig>,
    data: VecDeque<f32>,
    out: f32,
}

impl Delay {
    fn new(len: usize) -> Self {
        Delay {
            config: Arc::new(DelayConfig {
                samples: AtomicUsize::new(len),
                in_ty: AtomicU8::new(0),
            }),
            data: std::iter::repeat(0.0).take(len).collect(),
            out: 0.0,
        }
    }
}

impl Node for Delay {
    fn feed(&mut self, samples: &[Option<f32>]) -> Vec<NodeEvent> {
        let target_len = self.config.samples.load(Ordering::Relaxed);
        while target_len > self.data.len() {
            self.data.push_back(0.0);
        }
        if target_len < self.data.len() {
            self.data.drain(0..(self.data.len() - target_len));
        }

        self.data.push_back(samples[0].unwrap_or(0.0));
        self.out = self.data.pop_front().unwrap();

        Default::default()
    }

    fn read(&self) -> f32 {
        self.out
    }

    fn config(&self) -> Option<Arc<dyn NodeConfig>> {
        Some(Arc::clone(&self.config) as Arc<_>)
    }

    fn inputs(&self) -> Vec<Input> {
        vec![Input::new("sig")]
    }
}

pub fn delay() -> Box<dyn Node> {
    Box::new(Delay::new(4410))
}
