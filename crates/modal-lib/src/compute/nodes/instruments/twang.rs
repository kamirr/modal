use std::{
    f32::consts::PI,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};

use serde::{Deserialize, Serialize};

use crate::compute::{
    inputs::{freq::FreqInput, percentage::PercentageInput},
    nodes::all::delay::{RawDelay, ResizeStrategy},
};
use runtime::{
    node::{Input, Node, NodeConfig, NodeEvent},
    ExternInputs, Value, ValueKind,
};

static RANDOM: [f32; 20] = [
    -0.974_084_73,
    -0.231_807_25,
    -0.279_586_85,
    0.704_171,
    -0.259_593_13,
    0.125_517_44,
    -0.964_224_46,
    0.488_670_47,
    0.097_842_67,
    0.619_058_85,
    -0.042_867_195,
    -0.859_545_6,
    -0.629_87,
    0.733_124_7,
    -0.011_133_263,
    0.295_306_4,
    0.957_798_36,
    0.209_922_21,
    0.352_732_96,
    -0.213_263_26,
];

#[derive(Clone, Serialize, Deserialize, Debug)]
struct Fir2 {
    gain: f32,
    coeffs: [f32; 2],
    inputs: [f32; 2],
}

impl Fir2 {
    fn new(coeffs: [f32; 2]) -> Self {
        Fir2 {
            gain: 1.0,
            coeffs,
            inputs: [0.0; 2],
        }
    }

    fn tick(&mut self, input: f32) -> f32 {
        let [i0, _i1] = self.inputs;
        self.inputs = [self.gain * input, i0];

        let [i0, i1] = self.inputs;
        let [c0, c1] = self.coeffs;

        i0 * c0 + i1 * c1
    }

    fn phase_delay(&self, freq: f32) -> f32 {
        let omega_t = 2.0 * PI * freq / 44100.0;
        let mut real = 0f32;
        let mut imag = 0f32;

        for (i, coeff) in self.coeffs.iter().enumerate() {
            real += coeff * (omega_t * i as f32).cos();
            imag -= coeff * (omega_t * i as f32).sin();
        }

        real *= self.gain;
        imag *= self.gain;

        let mut phase = imag.atan2(real);

        real = 0.0;
        imag = 0.0;
        for i in 0..self.coeffs.len() {
            real += (omega_t * i as f32).cos();
            imag -= (omega_t * i as f32).sin();
        }

        phase -= imag.atan2(real);
        phase = (-phase) % (2.0 * PI);
        phase / omega_t
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct TwangConfig {
    pluck: AtomicBool,
}

impl NodeConfig for TwangConfig {
    fn show(&self, ui: &mut eframe::egui::Ui, _data: &dyn std::any::Any) {
        self.pluck.fetch_or(
            ui.centered_and_justified(|ui| ui.button("Pluck").clicked())
                .inner,
            Ordering::Relaxed,
        );
    }
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct Twang {
    config: Arc<TwangConfig>,
    pluck_pos_input: Arc<PercentageInput>,
    freq_input: Arc<FreqInput>,

    delay_line: RawDelay,
    comb_delay: RawDelay,
    loop_filt: Fir2,

    out: f32,
    freq: f32,
    loop_gain: f32,
    pluck_pos: f32,
}

impl Twang {
    pub fn new() -> Self {
        let mut this = Twang {
            config: Arc::new(TwangConfig {
                pluck: AtomicBool::new(false),
            }),
            pluck_pos_input: Arc::new(PercentageInput::new(40.0)),
            freq_input: Arc::new(FreqInput::new(220.0)),

            delay_line: RawDelay::new_allpass(4096.0),
            comb_delay: RawDelay::new_linear(4096.0),
            loop_filt: Fir2::new([0.5, 0.5]),

            out: 0.0,
            freq: 0.0,
            loop_gain: 0.995,
            pluck_pos: 0.4,
        };

        this.delay_line.resize_strategy(ResizeStrategy::Resample {
            freq_div: 44100 / 40, // resample 40 times per second
        });
        this.set_frequency(220.0);

        this
    }

    fn set_frequency(&mut self, freq: f32) {
        self.freq = freq;
        let delay = (44100.0 / freq) - self.loop_filt.phase_delay(freq);

        self.delay_line.resize(delay);
        self.set_loop_gain(self.loop_gain);
        self.comb_delay.resize(0.5 * self.pluck_pos * delay);
    }

    fn set_loop_gain(&mut self, loop_gain: f32) {
        self.loop_gain = loop_gain;
        let gain = loop_gain + (self.freq * 0.000005);
        let gain = gain.min(0.99999);
        self.loop_filt.gain = gain;
    }

    fn tick(&mut self, input: f32) {
        let filt_out = self.loop_filt.tick(self.delay_line.last_out());
        self.delay_line.push(input + filt_out);

        self.out = self.delay_line.last_out();

        self.comb_delay.push(self.out);
        self.out -= self.comb_delay.last_out();
        self.out *= 0.5;
    }
}

#[typetag::serde]
impl Node for Twang {
    fn feed(&mut self, _inputs: &ExternInputs, data: &[Value]) -> Vec<NodeEvent> {
        if self.config.pluck.fetch_and(false, Ordering::Relaxed) {
            for r in RANDOM.iter() {
                self.tick(*r);
            }
        }

        let new_freq = self.freq_input.get_f32(&data[1]);
        let new_pluck_pos = self.pluck_pos_input.get_f32(&data[2]);
        if new_freq != self.freq || new_pluck_pos != self.pluck_pos {
            self.pluck_pos = new_pluck_pos;
            self.set_frequency(new_freq);
        }

        self.tick(data[0].as_float().unwrap_or_default());

        Vec::default()
    }

    fn read(&self, out: &mut [Value]) {
        out[0] = Value::Float(self.out);
    }

    fn inputs(&self) -> Vec<Input> {
        vec![
            Input::new("sig", ValueKind::Float),
            Input::stateful("freq", &self.freq_input),
            Input::stateful("pluck at", &self.pluck_pos_input),
        ]
    }

    fn config(&self) -> Option<Arc<dyn NodeConfig>> {
        Some(Arc::clone(&self.config) as Arc<dyn NodeConfig>)
    }
}

pub fn twang() -> Box<dyn Node> {
    Box::new(Twang::new())
}
