use std::{
    collections::VecDeque,
    f32::consts::PI,
    sync::{
        atomic::{AtomicBool, AtomicU32, AtomicU8, AtomicUsize, Ordering},
        Arc,
    },
};

use eframe::egui::{ComboBox, DragValue};
use num_traits::{FromPrimitive, ToPrimitive};

use super::{
    inputs::{freq::FreqInput, positive::PositiveInput, real::RealInput, sig::SigInput},
    Input, InputUi, Node, NodeConfig, NodeEvent, NodeList,
};

#[derive(Clone, Debug)]
struct Constant {
    value: Arc<SigInput>,
    out: f32,
}

impl Node for Constant {
    fn feed(&mut self, data: &[Option<f32>]) -> Vec<NodeEvent> {
        self.out = data[0].unwrap_or(self.value.value());

        Default::default()
    }

    fn read(&self) -> f32 {
        self.out
    }

    fn inputs(&self) -> Vec<Input> {
        vec![Input {
            name: "value".into(),
            default_value: Some(Arc::clone(&self.value) as Arc<_>),
        }]
    }
}

pub fn constant() -> Box<dyn Node> {
    Box::new(Constant {
        value: Arc::new(SigInput::new(0.0)),
        out: 0.0,
    })
}

#[derive(Debug, PartialEq, Eq, num_derive::FromPrimitive, num_derive::ToPrimitive)]
enum OscType {
    Sine = 0,
    Square = 1,
}

#[derive(Debug)]
struct OscillatorConfig {
    ty: AtomicU8,
    manual_range: AtomicBool,
}

impl NodeConfig for OscillatorConfig {
    fn show(&self, ui: &mut eframe::egui::Ui) {
        let mut ty = OscType::from_u8(self.ty.load(Ordering::Acquire)).unwrap();
        let mut manual_range = self.manual_range.load(Ordering::Acquire);

        ComboBox::from_label("")
            .selected_text(format!("{ty:?}"))
            .show_ui(ui, |ui| {
                ui.selectable_value(&mut ty, OscType::Sine, "Sine");
                ui.selectable_value(&mut ty, OscType::Square, "Square")
            });
        ui.checkbox(&mut manual_range, "Manual range");

        self.ty.store(ty.to_u8().unwrap(), Ordering::Release);
        self.manual_range.store(manual_range, Ordering::Release);
    }
}

#[derive(Clone, Debug)]
struct Oscillator {
    config: Arc<OscillatorConfig>,
    f: Arc<FreqInput>,
    min: Arc<RealInput>,
    max: Arc<RealInput>,
    t: f32,
    out: f32,
    manual_range: bool,
}

impl Oscillator {
    fn hz_to_dt() -> f32 {
        2.0 * std::f32::consts::PI / 44100.0
    }
}

impl Node for Oscillator {
    fn feed(&mut self, data: &[Option<f32>]) -> Vec<NodeEvent> {
        let f = data[0].unwrap_or(self.f.value());
        let min = data
            .get(1)
            .unwrap_or(&Some(-1.0))
            .unwrap_or(self.min.value());
        let max = data
            .get(2)
            .unwrap_or(&Some(1.0))
            .unwrap_or(self.max.value());

        let step = f * Self::hz_to_dt();
        self.t = (self.t + step) % (8.0 * PI);

        let m1_to_p1 = match OscType::from_u8(self.config.ty.load(Ordering::Relaxed)).unwrap() {
            OscType::Sine => self.t.sin(),
            OscType::Square => {
                if self.t.sin() > 0.0 {
                    1.0
                } else {
                    -1.0
                }
            }
        };

        let zero_to_one = (m1_to_p1 + 1.0) / 2.0;
        self.out = zero_to_one * (max - min) + min;

        let manual_range = self.config.manual_range.load(Ordering::Relaxed);
        let emit_change = manual_range != self.manual_range;
        self.manual_range = manual_range;

        if emit_change {
            vec![NodeEvent::RecalcInputs(self.inputs())]
        } else {
            Default::default()
        }
    }

    fn read(&self) -> f32 {
        self.out
    }

    fn config(&self) -> Option<Arc<dyn NodeConfig>> {
        Some(Arc::clone(&self.config) as Arc<dyn NodeConfig>)
    }

    fn inputs(&self) -> Vec<Input> {
        let mut inputs = vec![Input {
            name: "f".into(),
            default_value: Some(Arc::clone(&self.f) as Arc<_>),
        }];
        if self.manual_range {
            inputs.extend([
                Input {
                    name: "min".into(),
                    default_value: Some(Arc::clone(&self.min) as Arc<_>),
                },
                Input {
                    name: "max".into(),
                    default_value: Some(Arc::clone(&self.max) as Arc<_>),
                },
            ])
        }

        inputs
    }
}

fn oscillator() -> Box<dyn Node> {
    Box::new(Oscillator {
        config: Arc::new(OscillatorConfig {
            ty: AtomicU8::new(OscType::Sine.to_u8().unwrap()),
            manual_range: AtomicBool::new(false),
        }),
        f: Arc::new(FreqInput::new(440.0)),
        min: Arc::new(RealInput::new(-1.0)),
        max: Arc::new(RealInput::new(1.0)),
        t: 0.0,
        out: 0.0,
        manual_range: false,
    })
}

#[derive(Clone, Debug)]
struct Gain {
    s1: Arc<PositiveInput>,
    out: f32,
}

impl Node for Gain {
    fn feed(&mut self, data: &[Option<f32>]) -> Vec<NodeEvent> {
        let s0 = data[0].unwrap_or(0.0);
        let s1 = data[1].unwrap_or(self.s1.value());

        self.out = s0 * s1;

        Default::default()
    }

    fn read(&self) -> f32 {
        self.out
    }

    fn inputs(&self) -> Vec<Input> {
        vec![
            Input {
                name: "sig#0".into(),
                default_value: None,
            },
            Input {
                name: "sig#1".into(),
                default_value: Some(Arc::clone(&self.s1) as Arc<_>),
            },
        ]
    }
}

fn gain() -> Box<dyn Node> {
    Box::new(Gain {
        s1: Arc::new(PositiveInput::new(1.0)),
        out: 0.0,
    })
}

#[derive(Debug)]
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

#[derive(Clone, Debug)]
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
        vec![Input {
            name: "sig".into(),
            default_value: None,
        }]
    }
}

pub fn delay() -> Box<dyn Node> {
    Box::new(Delay::new(4410))
}

#[derive(Debug)]
struct AddConfig {
    ins: AtomicU32,
}

impl NodeConfig for AddConfig {
    fn show(&self, ui: &mut eframe::egui::Ui) {
        let mut ins = self.ins.load(Ordering::Acquire);

        ui.add(DragValue::new(&mut ins).clamp_range(0..=std::u32::MAX));

        self.ins.store(ins, Ordering::Release);
    }
}

#[derive(Clone, Debug)]
pub struct Add {
    config: Arc<AddConfig>,
    ins: u32,
    out: f32,
}

impl Add {
    pub fn new(ins: u32) -> Self {
        Add {
            config: Arc::new(AddConfig {
                ins: AtomicU32::new(ins),
            }),
            ins,
            out: 0.0,
        }
    }
}

impl Node for Add {
    fn feed(&mut self, data: &[Option<f32>]) -> Vec<NodeEvent> {
        self.out = data.iter().map(|opt| opt.unwrap_or(0.0)).sum();

        let new_ins = self.config.ins.load(Ordering::Relaxed);
        let emit_ev = new_ins != self.ins;
        self.ins = new_ins;

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
            .map(|i| Input {
                name: format!("sig#{i}"),
                default_value: None,
            })
            .collect()
    }
}

fn add() -> Box<dyn Node> {
    Box::new(Add::new(2))
}

pub struct Basic;

impl NodeList for Basic {
    fn all(&self) -> Vec<(fn() -> Box<dyn Node>, &'static str)> {
        vec![
            (add, "Add"),
            (constant, "Constant"),
            (delay, "Delay"),
            (oscillator, "Oscillator"),
            (gain, "Gain"),
        ]
    }
}
