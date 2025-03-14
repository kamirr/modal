use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, RwLock, RwLockReadGuard, RwLockWriteGuard,
};

use crate::compute::inputs::{
    gate::{GateInput, GateInputState},
    real::RealInput,
    time::TimeInput,
    trigger::{TriggerInput, TriggerInputState, TriggerMode},
};
use eframe::{
    egui,
    epaint::{Color32, Vec2},
};
use egui_curve_edit as egui_curve;
use runtime::{
    node::{Input, Node, NodeConfig, NodeEvent},
    ExternInputs, Value,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct CurveConfig {
    curve: RwLock<egui_curve::Curve>,
    sampled: RwLock<Vec<f32>>,
    edit: AtomicBool,
}

impl Default for CurveConfig {
    fn default() -> Self {
        Self::new()
    }
}

impl CurveConfig {
    pub fn new() -> Self {
        CurveConfig {
            curve: RwLock::new(egui_curve::Curve::new([0.0, 50.0], [100.0, 50.0])),
            sampled: RwLock::new(vec![0.0, 0.0, 0.0]),
            edit: AtomicBool::new(false),
        }
    }

    pub fn values(&self) -> RwLockReadGuard<'_, Vec<f32>> {
        self.sampled.read().unwrap()
    }

    pub fn values_mut(&self) -> RwLockWriteGuard<'_, Vec<f32>> {
        self.sampled.write().unwrap()
    }
}

impl NodeConfig for CurveConfig {
    fn show(&self, ui: &mut egui::Ui, _data: &dyn std::any::Any) {
        let mut edit = self.edit.load(Ordering::Acquire);

        egui::CollapsingHeader::new("Shape").show(ui, |ui| {
            ui.vertical(|ui| {
                let button = if edit {
                    egui::Button::new(egui::RichText::new("Edit").color(Color32::BLACK))
                        .fill(Color32::GOLD)
                } else {
                    egui::Button::new("Edit")
                }
                .min_size(Vec2::new(ui.available_width(), 0.0));

                if ui.add(button).clicked() {
                    edit = !edit;
                }

                let values = self.values();
                let xys: Vec<_> = self
                    .values()
                    .iter()
                    .enumerate()
                    .map(|(i, y)| [i as f64 / (values.len() - 1) as f64, *y as f64])
                    .collect();

                let line = egui_plot::Line::new(xys);

                egui_plot::Plot::new("plot")
                    .show_x(false)
                    .show_y(false)
                    .allow_zoom(false)
                    .allow_scroll(false)
                    .allow_boxed_zoom(false)
                    .allow_drag(false)
                    .view_aspect(2.0)
                    .show_axes([false, false])
                    .include_x(0.0)
                    .include_x(1.0)
                    .include_y(0.0)
                    .include_y(100.0)
                    .show(ui, |ui| {
                        ui.line(line);
                    });
            });
        });

        if edit {
            let mut curve = self.curve.write().unwrap();

            egui::Window::new("Curve").show(ui.ctx(), |ui| {
                ui.add(egui_curve::CurveEdit::new(&mut curve, 0.0..=100.0));
            });

            *self.values_mut() = curve.sample_along_x(256, 0.0..=100.0);
        }

        self.edit.store(edit, Ordering::Release);
    }

    fn show_short(&self, ui: &mut egui::Ui, data: &dyn std::any::Any) {
        self.show(ui, data);
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
enum CurveStatus {
    Playing,
    Done,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Curve {
    config: Arc<CurveConfig>,

    trigger: Arc<TriggerInput>,
    trigger_state: TriggerInputState,
    length: Arc<TimeInput>,
    min: Arc<RealInput>,
    max: Arc<RealInput>,
    repeat: Arc<GateInput>,
    repeat_state: GateInputState,
    resettable: Arc<GateInput>,
    resettable_state: GateInputState,

    status: CurveStatus,
    t: f32,
    out: f32,
}

impl Default for Curve {
    fn default() -> Self {
        Self::new()
    }
}

impl Curve {
    pub fn new() -> Self {
        Curve {
            config: Arc::new(CurveConfig::new()),

            trigger: Arc::new(TriggerInput::new(TriggerMode::Up, 0.5)),
            trigger_state: TriggerInputState::default(),
            length: Arc::new(TimeInput::new(44100.0)),
            min: Arc::new(RealInput::new(-1.0)),
            max: Arc::new(RealInput::new(1.0)),
            repeat: Arc::new(GateInput::new(0.5)),
            repeat_state: GateInputState::default(),
            resettable: Arc::new(GateInput::new(0.5)),
            resettable_state: GateInputState::default(),

            status: CurveStatus::Done,
            t: 0.0,
            out: 0.0,
        }
    }
}

#[typetag::serde]
impl Node for Curve {
    fn feed(&mut self, _inputs: &ExternInputs, data: &[Value]) -> Vec<NodeEvent> {
        let trigger = self.trigger.trigger(&mut self.trigger_state, &data[0]);
        let length = self.length.get_samples(&data[1]);
        let min = self.min.get_f32(&data[2]);
        let max = self.max.get_f32(&data[3]);
        let repeat = self.repeat.gate(&mut self.repeat_state, &data[4]);
        let resettable = self.resettable.gate(&mut self.resettable_state, &data[5]);
        if trigger && (self.status == CurveStatus::Done || resettable) {
            self.status = CurveStatus::Playing;
            self.t = 0.0;
        }

        if self.t > length {
            self.status = CurveStatus::Done;
        }

        if self.status == CurveStatus::Done && repeat {
            self.status = CurveStatus::Playing;
            self.t = 0.0;
        }

        let raw_out = match self.status {
            CurveStatus::Done => self.config.values()[0],
            CurveStatus::Playing => {
                let values = self.config.values();
                let t = self.t / length;

                let idx_f32 = t * values.len() as f32;
                let idx = idx_f32 as usize;
                let idx = idx.clamp(0, values.len() - 2);

                let curr = values[idx];
                let next = values[idx + 1];
                let f = idx_f32 - idx as f32;

                curr * (1.0 - f) + next * f
            }
        };

        self.out = raw_out / 100.0 * (max - min) + min;
        self.t += 1.0;

        Default::default()
    }

    fn read(&self, out: &mut [Value]) {
        out[0] = Value::Float(self.out)
    }

    fn config(&self) -> Option<Arc<dyn NodeConfig>> {
        Some(Arc::clone(&self.config) as Arc<_>)
    }

    fn inputs(&self) -> Vec<Input> {
        vec![
            Input::stateful("trigger", &self.trigger),
            Input::stateful("length", &self.length),
            Input::stateful("min", &self.min),
            Input::stateful("max", &self.max),
            Input::stateful("repeat", &self.repeat),
            Input::stateful("resettable", &self.resettable),
        ]
    }
}

pub fn curve() -> Box<dyn Node> {
    Box::new(Curve::new())
}
