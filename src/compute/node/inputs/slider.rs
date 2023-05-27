use atomic_float::AtomicF32;
use eframe::egui;
use serde::{Deserialize, Serialize};
use std::sync::atomic::Ordering;

use crate::compute::{node::InputUi, Value};

#[derive(Debug, Serialize, Deserialize)]
pub struct SliderInput {
    s: AtomicF32,
    min: f32,
    max: f32,
    integral: bool,
    show_connected: bool,
}

impl SliderInput {
    pub fn new(f: f32, min: f32, max: f32) -> Self {
        SliderInput {
            s: AtomicF32::new(f),
            min,
            max,
            integral: false,
            show_connected: false,
        }
    }

    pub fn show_connected(mut self, value: bool) -> Self {
        self.show_connected = value;
        self
    }

    pub fn integral(mut self, value: bool) -> Self {
        self.integral = value;
        self
    }

    pub fn as_f32(&self, recv: &Value) -> f32 {
        recv.as_float().unwrap_or(self.s.load(Ordering::Relaxed))
    }

    fn show(&self, ui: &mut egui::Ui) {
        let mut s = self.s.load(Ordering::Acquire);

        let mut slider = egui::Slider::new(&mut s, self.min..=self.max);
        if self.integral {
            slider = slider.fixed_decimals(0).step_by(1.0).drag_value_speed(1.0);
        }

        ui.add(slider);

        self.s.store(s, Ordering::Release);
    }
}

impl InputUi for SliderInput {
    fn show_disconnected(&self, ui: &mut eframe::egui::Ui, _verbose: bool) {
        if !self.show_connected {
            self.show(ui);
        }
    }

    fn show_always(&self, ui: &mut egui::Ui, _verbose: bool) {
        if self.show_connected {
            self.show(ui);
        }
    }
}
