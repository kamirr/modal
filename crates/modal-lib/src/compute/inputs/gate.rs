use std::sync::atomic::{AtomicBool, Ordering};

use atomic_float::AtomicF32;
use runtime::{node::InputUi, Value, ValueKind};
use serde::{Deserialize, Serialize};

use crate::serde_atomic_enum;

use super::real::RealInput;

#[atomic_enum::atomic_enum]
#[derive(PartialEq, Eq)]
enum Edge {
    Positive,
    Negative,
    None,
}

serde_atomic_enum!(AtomicEdge);

#[derive(Debug, Serialize, Deserialize)]
pub struct GateInput {
    threshold: RealInput,
    default: AtomicBool,
    edge: AtomicEdge,
    prev: AtomicF32,
}

impl GateInput {
    pub fn new(threshold: f32) -> Self {
        GateInput {
            threshold: RealInput::new(threshold),
            default: AtomicBool::new(false),
            edge: AtomicEdge::new(Edge::None),
            prev: AtomicF32::new(0.0),
        }
    }

    pub fn positive_edge(&self) -> bool {
        self.edge.load(Ordering::Relaxed) == Edge::Positive
    }

    pub fn negative_edge(&self) -> bool {
        self.edge.load(Ordering::Relaxed) == Edge::Negative
    }

    pub fn gate(&self, recv: &Value) -> bool {
        let prev = self.prev.load(Ordering::Acquire);
        let default = self.default.load(Ordering::Acquire);
        let threshold = self.threshold.get_f32(&Value::None);

        let curr = recv.as_float().unwrap_or(if default {
            threshold + 1.0
        } else {
            threshold - 1.0
        });

        let edge = if curr >= threshold && prev < threshold {
            Edge::Positive
        } else if curr < threshold && prev >= threshold {
            Edge::Negative
        } else {
            Edge::None
        };

        self.edge.store(edge, Ordering::Relaxed);
        self.prev.store(curr, Ordering::Release);

        curr >= threshold
    }
}

impl InputUi for GateInput {
    fn value_kind(&self) -> ValueKind {
        ValueKind::Float
    }

    fn show_always(&self, ui: &mut eframe::egui::Ui, verbose: bool) {
        if verbose {
            self.threshold.show_disconnected(ui, verbose);
        }
    }

    fn show_disconnected(&self, ui: &mut eframe::egui::Ui, _verbose: bool) {
        let mut default = self.default.load(Ordering::Acquire);
        ui.checkbox(&mut default, "default");

        self.default.store(default, Ordering::Release);
    }
}
