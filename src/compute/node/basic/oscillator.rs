use std::{
    f32::consts::PI,
    sync::{
        atomic::{AtomicBool, AtomicU8, Ordering},
        Arc,
    },
};

use eframe::egui::ComboBox;
use num_traits::{FromPrimitive, ToPrimitive};

use crate::compute::node::{
    inputs::{freq::FreqInput, real::RealInput},
    Input, InputUi, Node, NodeConfig, NodeEvent,
};

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
        let mut inputs = vec![Input::with_default("f", &self.f)];
        if self.manual_range {
            inputs.extend([
                Input::with_default("min", &self.min),
                Input::with_default("max", &self.max),
            ])
        }

        inputs
    }
}

pub fn oscillator() -> Box<dyn Node> {
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
