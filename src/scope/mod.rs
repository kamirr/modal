use std::{collections::VecDeque, fmt::Debug};

use eframe::egui::{
    self,
    plot::{Line, Plot, PlotPoints},
};
use num_traits::Zero;
use rustfft::{num_complex::Complex32, FftPlanner};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ScopeMode {
    TimeSeries,
    Fft,
}

struct MyPlanner(FftPlanner<f32>);

impl Default for MyPlanner {
    fn default() -> Self {
        MyPlanner(FftPlanner::new())
    }
}

impl Clone for MyPlanner {
    fn clone(&self) -> Self {
        MyPlanner::default()
    }
}

impl Debug for MyPlanner {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MyPlanner").finish()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Scope {
    // general
    mode: ScopeMode,

    // fft
    freq_range: (usize, usize),

    memory: VecDeque<f32>,
    rolling_min: VecDeque<f32>,
    rolling_max: VecDeque<f32>,
    rolling_len: usize,
    #[serde(skip)]
    fft_planner: MyPlanner,
    #[serde(skip)]
    scratch: Vec<Complex32>,
}

impl Scope {
    pub fn new() -> Self {
        let rolling_len = 180;
        Scope {
            mode: ScopeMode::TimeSeries,
            freq_range: (100, 5000),
            memory: std::iter::repeat(0.0).take(44100).collect(),
            rolling_min: std::iter::repeat(-1.0).take(rolling_len).collect(),
            rolling_max: std::iter::repeat(1.0).take(rolling_len).collect(),
            rolling_len,
            fft_planner: MyPlanner(FftPlanner::new()),
            scratch: Vec::new(),
        }
    }

    fn show_timeseries(&self, ui: &mut egui::Ui) {
        let len_t = self.memory.len() as f32 / 44100.0;
        let xys: PlotPoints = self
            .memory
            .iter()
            .enumerate()
            .step_by(44)
            .map(|(i, y)| {
                let t = i as f32 / 44100.0 - len_t;

                [t as f64, *y as f64]
            })
            .collect();

        let min = xys.points()[0].x - 0.1;
        let max = xys.points().last().unwrap().x + 0.1;
        let line = Line::new(xys);

        let min_y = self
            .rolling_min
            .iter()
            .copied()
            .min_by(|a, b| a.partial_cmp(b).unwrap())
            .unwrap() as f64;

        let max_y = self
            .rolling_max
            .iter()
            .copied()
            .max_by(|a, b| a.partial_cmp(b).unwrap())
            .unwrap() as f64;

        let h = max_y - min_y;

        Plot::new("plot")
            .include_x(min)
            .include_x(max)
            .include_y(min_y - h / 10.0)
            .include_y(max_y + h / 10.0)
            .show_x(false)
            .show_y(false)
            .allow_zoom(false)
            .allow_scroll(false)
            .allow_boxed_zoom(false)
            .allow_drag(false)
            .view_aspect(2.0)
            .show(ui, |ui| {
                ui.line(line);
            });
    }

    fn show_fft(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label("From");
            ui.add(
                egui::DragValue::new(&mut self.freq_range.0).clamp_range(1..=self.freq_range.1 - 1),
            );
            ui.label("Hz to");
            ui.add(
                egui::DragValue::new(&mut self.freq_range.1)
                    .clamp_range(self.freq_range.0 + 1..=12000),
            );
            ui.label("Hz");
        });

        let mut ys = self
            .memory
            .iter()
            .map(|v| Complex32 { re: *v, im: 0.0 })
            .collect::<Vec<_>>();

        self.scratch.resize(ys.len(), Complex32::zero());
        self.fft_planner
            .0
            .plan_fft_forward(ys.len())
            .process_with_scratch(&mut ys, &mut self.scratch);

        let hz_per_i = 44100.0 / (ys.len() as f32);
        let start_i = (self.freq_range.0 as f32 / hz_per_i).round() as usize;
        let end_i = (self.freq_range.1 as f32 / hz_per_i).round() as usize;
        let xys: PlotPoints = ys
            .iter()
            .enumerate()
            .skip(start_i)
            .take(end_i - start_i + 1)
            .map(|(i, y)| {
                let f = i as f64 * (hz_per_i as f64);

                [f, y.norm() as f64]
            })
            .collect();

        let first_y = xys.points()[0].y;

        let min = xys.points()[0].x - 0.1;
        let max = xys.points().last().unwrap().x + 0.1;
        let line = Line::new(xys);

        Plot::new("plot")
            .include_x(min)
            .include_x(max)
            .include_y(first_y)
            .show_x(false)
            .show_y(false)
            .allow_zoom(false)
            .allow_scroll(false)
            .allow_boxed_zoom(false)
            .allow_drag(false)
            .view_aspect(2.0)
            .show(ui, |ui| {
                ui.line(line);
            });
    }

    pub fn show(&mut self, ui: &mut egui::Ui) {
        let mut mem_s = self.memory.len() as f32 / 44100.0;
        let drag = egui::DragValue::new(&mut mem_s)
            .speed(0.01)
            .clamp_range(0.01..=120.0);
        ui.horizontal(|ui| {
            ui.label("memory");
            ui.add(drag);
            ui.label("s");
        });

        let new_mem = (mem_s * 44100.0).round() as usize;
        if new_mem < self.memory.len() {
            self.memory.drain(0..self.memory.len() - new_mem);
        } else {
            for _ in 0..(new_mem - self.memory.len()) {
                self.memory.push_front(0.0);
            }
        }

        egui::ComboBox::new("scope-combo-box", "")
            .selected_text(format!("{:?}", self.mode))
            .show_ui(ui, |ui| {
                ui.selectable_value(&mut self.mode, ScopeMode::TimeSeries, "TimeSeries");
                ui.selectable_value(&mut self.mode, ScopeMode::Fft, "Fft")
            });

        match self.mode {
            ScopeMode::TimeSeries => self.show_timeseries(ui),
            ScopeMode::Fft => self.show_fft(ui),
        }
    }

    pub fn feed(&mut self, data: impl Iterator<Item = f32>) {
        for pt in data {
            self.memory.pop_front();
            self.memory.push_back(pt);
        }
        self.rolling_min.push_front(
            self.memory
                .iter()
                .copied()
                .min_by(|a, b| a.partial_cmp(b).unwrap())
                .unwrap(),
        );

        self.rolling_max.push_front(
            self.memory
                .iter()
                .copied()
                .max_by(|a, b| a.partial_cmp(b).unwrap())
                .unwrap(),
        );

        if self.rolling_min.len() > self.rolling_len {
            self.rolling_min.pop_back();
        }
        if self.rolling_max.len() > self.rolling_len {
            self.rolling_max.pop_back();
        }
    }
}

impl Default for Scope {
    fn default() -> Self {
        Scope::new()
    }
}
