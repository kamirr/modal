use atomic_enum::atomic_enum;
use atomic_float::AtomicF32;
use eframe::egui;
use egui_plot::{GridMark, Legend, Line, Plot, PlotPoints, VLine};
use num_complex::Complex32;
use serde::{Deserialize, Serialize};

use crate::{
    compute::{Value, ValueKind},
    serde_atomic_enum,
    util::{enum_combo_box, toggle_button},
};

use crate::node::{
    inputs::{freq::FreqInput, positive::PositiveInput},
    Input, Node, NodeConfig, NodeEvent,
};

use std::{
    any::Any,
    f32::consts::PI,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
};

#[atomic_enum]
#[derive(PartialEq, derive_more::Display, strum::EnumIter)]
pub enum BiquadTy {
    Low,
    High,
    Band,
    Notch,
    All,
}

serde_atomic_enum!(AtomicBiquadTy);

#[atomic_enum]
#[derive(PartialEq, Serialize, Deserialize, derive_more::Display, strum::EnumIter)]
enum ParamTy {
    Q,
    Bw,
}

serde_atomic_enum!(AtomicParamTy);

#[derive(Debug, Serialize, Deserialize)]
struct BiquadConfig {
    filt_ty: AtomicBiquadTy,
    param_ty: AtomicParamTy,
    // For bode plot
    show_plot: AtomicBool,
    freq: AtomicF32,
    update_coeffs: AtomicBool,
    coeffs: Mutex<([f32; 3], [f32; 3])>,
}

impl BiquadConfig {
    fn new(filt_ty: BiquadTy, param_ty: ParamTy, freq: f32) -> Self {
        BiquadConfig {
            filt_ty: AtomicBiquadTy::new(filt_ty),
            param_ty: AtomicParamTy::new(param_ty),
            show_plot: AtomicBool::new(false),
            freq: AtomicF32::new(freq),
            update_coeffs: AtomicBool::new(true),
            coeffs: Mutex::new(([0.0; 3], [0.0; 3])),
        }
    }

    fn show_plot(&self, ui: &mut egui::Ui) {
        self.update_coeffs.store(true, Ordering::Relaxed);
        let (a, b) = *self.coeffs.lock().unwrap();
        let f = self.freq.load(Ordering::Relaxed);

        let a = a.map(|f| Complex32::new(f, 0.0));
        let b = b.map(|f| Complex32::new(f, 0.0));

        let max_f = (f as u32 * 3).max(1000).min(20000) / 1000 * 1000;

        let mut xs: Vec<_> = (0..max_f)
            .step_by(max_f as usize / 120)
            .skip(1)
            .map(|f| f as f32)
            .chain([1.0, f - 0.001, f, f + 0.001, max_f as f32])
            .collect();
        xs.sort_by(|a, b| a.total_cmp(b));

        let samples: Vec<(f32, Complex32)> = xs
            .iter()
            .copied()
            .map(|f| {
                let w = 2.0 * PI * f;
                let s = Complex32::new(0.0, w);
                let z = (s * (1.0 / 44100.0)).exp();

                let h = (b[0] + b[1] / z + b[2] / z / z) / (a[0] + a[1] / z + a[2] / z / z);

                (f, h)
            })
            .collect();

        let magn_xys = PlotPoints::new(
            samples
                .iter()
                .copied()
                .map(|(f, h)| [f as _, h.norm() as _])
                .collect(),
        );
        let phase_xys = PlotPoints::new(
            samples
                .iter()
                .copied()
                .map(|(f, h)| [f as _, (h.arg() / PI * 180.0) as _])
                .collect(),
        );

        egui::Window::new("Bode Plot").show(ui.ctx(), |ui| {
            ui.vertical(|ui| {
                let plots = [
                    (
                        Line::new(magn_xys),
                        "Magnitude",
                        (0.0, 1.0),
                        (|mark: GridMark, _| format!("{:.2}", mark.value))
                            as for<'a> fn(_, &'a _) -> _,
                    ),
                    (
                        Line::new(phase_xys),
                        "Phase",
                        (-180.0, 180.0),
                        |mark: GridMark, _| format!("{:.0}°", mark.value),
                    ),
                ];
                for (line, name, (y_min, y_max), y_fmt) in plots {
                    ui.label(name);
                    Plot::new(name)
                        .include_y(y_min)
                        .include_y(y_max)
                        .x_axis_label("frequency")
                        .x_axis_formatter(|mark, _range| format!("{:.0}kHz", mark.value / 1000.0))
                        .y_axis_formatter(y_fmt)
                        .show_x(false)
                        .show_y(false)
                        .allow_zoom(false)
                        .allow_scroll(false)
                        .allow_boxed_zoom(false)
                        .allow_drag(false)
                        .view_aspect(2.0)
                        .legend(Legend::default())
                        .show(ui, |ui| {
                            ui.vline(VLine::new(f).name(format!("f0={f:}")));
                            ui.line(line);
                        });
                }
            });
        });
    }
}

impl NodeConfig for BiquadConfig {
    fn show(&self, ui: &mut egui::Ui, _data: &dyn Any) {
        let mut filt_ty = self.filt_ty.load(Ordering::Acquire);
        let mut param_ty = self.param_ty.load(Ordering::Acquire);
        let mut show_plot = self.show_plot.load(Ordering::Relaxed);

        ui.horizontal(|ui| {
            ui.label("Type");
            enum_combo_box(ui, &mut filt_ty);
        });
        ui.horizontal(|ui| {
            ui.label("Parameter");
            enum_combo_box(ui, &mut param_ty);
        });

        ui.centered_and_justified(|ui| {
            if ui.add(toggle_button("Show Bode Plot", show_plot)).clicked() {
                show_plot = !show_plot;
            }
        });

        if show_plot {
            self.show_plot(ui);
        }

        self.filt_ty.store(filt_ty, Ordering::Release);
        self.param_ty.store(param_ty, Ordering::Release);
        self.show_plot.store(show_plot, Ordering::Relaxed);
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Biquad {
    config: Arc<BiquadConfig>,
    f0: Arc<FreqInput>,
    q: Arc<PositiveInput>,
    bw: Arc<PositiveInput>,
    param_ty: ParamTy,
    in_hist: [f32; 3],
    out_hist: [f32; 2],
}

impl Biquad {
    pub fn new(ty: BiquadTy, freq: f32) -> Self {
        let config = BiquadConfig::new(ty, ParamTy::Q, freq);
        Biquad {
            config: Arc::new(config),
            f0: Arc::new(FreqInput::new(freq)),
            q: Arc::new(PositiveInput::new(0.707)),
            bw: Arc::new(PositiveInput::new(1.0)),
            param_ty: ParamTy::Q,
            in_hist: [0.0; 3],
            out_hist: [0.0; 2],
        }
    }

    fn next(&mut self, input: f32, f0: &Value, param: &Value) {
        let (a, b) = self.coeffs(f0, param);

        if self.config.update_coeffs.swap(false, Ordering::Relaxed) {
            *self.config.coeffs.lock().unwrap() = (a, b);
        }

        self.in_hist = [self.in_hist[1], self.in_hist[2], input];
        let out = (b[0] / a[0]) * self.in_hist[2]
            + (b[1] / a[0]) * self.in_hist[1]
            + (b[2] / a[0]) * self.in_hist[0]
            - (a[1] / a[0]) * self.out_hist[1]
            - (a[2] / a[0]) * self.out_hist[0];

        self.out_hist = [self.out_hist[1], out];
    }

    fn coeffs(&self, f0: &Value, param: &Value) -> ([f32; 3], [f32; 3]) {
        let ty = self.config.filt_ty.load(Ordering::Relaxed);
        let param_ty = self.config.param_ty.load(Ordering::Relaxed);

        let f0 = self.f0.get_f32(f0);
        self.config.freq.store(f0, Ordering::Relaxed);

        let param = match param_ty {
            ParamTy::Q => self.q.get_f32(param),
            ParamTy::Bw => self.bw.get_f32(param),
        };

        let w0 = 2.0 * PI * f0 / 44100.0;
        let w0sin = w0.sin();
        let w0cos = w0.cos();

        let alpha = match param_ty {
            ParamTy::Q => w0sin / 2.0 / param,
            ParamTy::Bw => w0sin * (2f32.ln() / 2.0 * param * w0 / w0sin).sinh(),
        };

        match ty {
            BiquadTy::Low => (
                [1.0 + alpha, -2.0 * w0cos, 1.0 - alpha],
                [(1.0 - w0cos) / 2.0, 1.0 - w0cos, (1.0 - w0cos) / 2.0],
            ),
            BiquadTy::High => (
                [1.0 + alpha, -2.0 * w0cos, 1.0 - alpha],
                [(1.0 + w0cos) / 2.0, -1.0 - w0cos, (1.0 + w0cos) / 2.0],
            ),
            BiquadTy::Band => (
                [1.0 + alpha, -2.0 * w0cos, 1.0 - alpha],
                [alpha, 0.0, -alpha],
            ),
            BiquadTy::Notch => (
                [1.0 + alpha, -2.0 * w0cos, 1.0 - alpha],
                [1.0, -2.0 * w0cos, 1.0],
            ),
            BiquadTy::All => (
                [1.0 + alpha, -2.0 * w0cos, 1.0 - alpha],
                [1.0 - alpha, -2.0 * w0cos, 1.0 + alpha],
            ),
        }
    }
}

#[typetag::serde]
impl Node for Biquad {
    fn feed(&mut self, data: &[Value]) -> Vec<NodeEvent> {
        self.next(data[0].as_float().unwrap_or_default(), &data[1], &data[2]);

        let new_param_ty = self.config.param_ty.load(Ordering::Relaxed);
        if self.param_ty != new_param_ty {
            self.param_ty = new_param_ty;

            vec![NodeEvent::RecalcInputs(self.inputs())]
        } else {
            Default::default()
        }
    }

    fn read(&self, out: &mut [Value]) {
        out[0] = Value::Float(self.out_hist[1])
    }
    fn config(&self) -> Option<Arc<dyn NodeConfig>> {
        Some(Arc::clone(&self.config) as Arc<_>)
    }

    fn inputs(&self) -> Vec<Input> {
        vec![
            Input::new("sig", ValueKind::Float),
            Input::stateful("f0", &self.f0),
            match &self.param_ty {
                ParamTy::Q => Input::stateful("Q", &self.q),
                ParamTy::Bw => Input::stateful("BW", &self.bw),
            },
        ]
    }
}

pub fn biquad() -> Box<dyn Node> {
    Box::new(Biquad::new(BiquadTy::Low, 440.0))
}
