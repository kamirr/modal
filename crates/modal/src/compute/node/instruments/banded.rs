use std::sync::Arc;

use serde::{Deserialize, Serialize};

use crate::compute::{
    node::{
        all::{
            biquad::{Biquad, BiquadTy},
            delay::RawDelay,
        },
        inputs::{
            freq::FreqInput,
            positive::PositiveInput,
            trigger::{TriggerInput, TriggerMode},
        },
        Input, Node, NodeEvent, NodeExt,
    },
    Value,
};

/// This class implements a simple bowed string non-linear function,
/// as described by Smith (1986).  The output is an instantaneous reflection
/// coefficient value. by Perry R. Cook and Gary P. Scavone, 1995--2023.
///
/// Ported from https://github.com/thestk/stk
#[derive(Serialize, Deserialize, Debug, Clone)]
struct BowTable {
    offset: f32,
    slope: f32,
    clamp_output: (f32, f32),
}

impl BowTable {
    fn compute(&mut self, input: f32) -> f32 {
        let mut sample = input + self.offset;
        sample *= self.slope;

        (sample.abs() + 0.75)
            .powi(-4)
            .clamp(self.clamp_output.0, self.clamp_output.1)
    }
}

impl Default for BowTable {
    fn default() -> Self {
        BowTable {
            offset: 0.0,
            slope: 0.1,
            clamp_output: (0.01, 0.98),
        }
    }
}

/// Banded Waveguide Modeling
///
/// This struct uses banded waveguide techniques to model a variety of sounds,
/// including bowed bars, glasses, and bowls.  For more information, see Essl,
/// G. and Cook, P. "Banded Waveguides: Towards Physical Modelling of Bar
/// Percussion Instruments", Proceedings of the 1999 International Computer
/// Music Conference.
///
/// Ported from https://github.com/thestk/stk
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Banded {
    pluck: Arc<TriggerInput>,
    bow_pressure: Arc<PositiveInput>,
    freq: Arc<FreqInput>,

    bow_vel: f32,
    bow_table: BowTable,

    modes: Vec<Mode>,
    base_gain: f32,
    integration_const: f32,
    strike_amp: f32,
    strike_pos: usize,

    curr_freq: f32,
    output: f32,
}

impl Banded {
    pub fn new(preset: BandedPreset) -> Self {
        const DEFAULT_FREQ: f32 = 220.0;

        Banded {
            pluck: Arc::new(TriggerInput::new(TriggerMode::Beat, 0.0)),
            bow_pressure: Arc::new(PositiveInput::new(0.0)),
            freq: Arc::new(FreqInput::new(DEFAULT_FREQ)),

            bow_vel: 0.0,
            bow_table: BowTable {
                slope: 3.0,
                ..Default::default()
            },

            modes: Mode::from_preset(preset, DEFAULT_FREQ),

            strike_amp: 0.0,

            base_gain: 0.999,
            integration_const: 0.0,

            strike_pos: 0,

            curr_freq: DEFAULT_FREQ,
            output: 0.0,
        }
    }

    fn pluck(&mut self, pluck: f32) {
        let min_len = self.modes[self.modes.len() - 1].delay.len();
        let modes_len = self.modes.len() as f32;
        for mode in &mut self.modes {
            let multiple = mode.delay.len() / min_len;
            for _ in 0..multiple as usize {
                let value = mode.excitation * pluck / modes_len;
                mode.delay.push(value);
            }
        }
    }
}

#[typetag::serde]
impl Node for Banded {
    fn feed(&mut self, data: &[Value]) -> Vec<NodeEvent> {
        let freq = self.freq.get_f32(&data[2]);
        if freq != self.curr_freq {
            self.curr_freq = freq;
            Mode::set_frequency(self.modes.as_mut_slice(), freq);
        }

        self.bow_vel = self.integration_const * self.bow_vel;
        self.bow_vel += self.base_gain
            * self
                .modes
                .iter()
                .map(|Mode { delay, .. }| delay.last_out())
                .sum::<f32>();

        let bow_pressure = self.bow_pressure.get_f32(&data[1]);
        let bow_en = bow_pressure > 0.0;
        let mut bow_input = bow_pressure - self.bow_vel;
        bow_input *= self.bow_table.compute(bow_input);
        bow_input /= self.modes.len() as f32;

        if self.pluck.trigger(&data[0]) {
            self.pluck(0.5)
        };

        self.output = self.modes.iter_mut().fold(0.0, |acc, mode| {
            let mut filt_in = mode.basegain * mode.delay.last_out();
            if bow_en {
                filt_in += bow_input;
            }

            mode.bandpass.feed(&[
                Value::Float(filt_in),
                Value::Disconnected,
                Value::Disconnected,
            ]);

            let filt_out = mode.bandpass.read_f32();
            mode.delay.push(filt_out);

            acc + filt_out
        }) * 4.0;

        Vec::default()
    }

    fn read(&self, out: &mut [Value]) {
        out[0] = Value::Float(self.output);
    }

    fn inputs(&self) -> Vec<Input> {
        vec![
            Input::stateful("pluck", &self.pluck),
            Input::stateful("bow", &self.bow_pressure),
            Input::stateful("freq", &self.freq),
        ]
    }
}

/// Implemented presets of [`Banded`]
pub enum BandedPreset {
    TunedBar,
    GlassHarmonica,
    TibetanPrayerBowl,
    UniformBar,
}

/// Single Band of the Banded Waveguide Model
#[derive(Clone, Serialize, Deserialize, Debug)]
struct Mode {
    mode: f32,
    basegain: f32,
    excitation: f32,
    bandpass: Biquad,
    delay: RawDelay,
}

impl Mode {
    fn from_preset(preset: BandedPreset, freq: f32) -> Vec<Self> {
        let base_len = 44100.0 / freq;

        match preset {
            BandedPreset::TunedBar => core::array::from_fn::<_, 4, _>(|i| {
                let mode = [1.0, 4.0198391420, 10.7184986595, 18.0697050938][i];
                let delay_len = base_len / mode;

                Mode {
                    mode,
                    basegain: 0.999f32.powi(i as i32 + 1),
                    excitation: 1.0,
                    bandpass: Biquad::new(BiquadTy::Band, freq * mode),
                    delay: RawDelay::new_linear(delay_len),
                }
            })
            .to_vec(),
            BandedPreset::GlassHarmonica => core::array::from_fn::<_, 5, _>(|i| {
                let mode = [1.0, 2.32, 4.25, 6.63, 9.38][i];
                let delay_len = base_len / mode;

                Mode {
                    mode,
                    basegain: 0.999f32.powi(i as i32 + 1),
                    excitation: 1.0,
                    bandpass: Biquad::new(BiquadTy::Band, freq * mode),
                    delay: RawDelay::new_linear(delay_len),
                }
            })
            .to_vec(),
            BandedPreset::TibetanPrayerBowl => core::array::from_fn::<_, 12, _>(|i| {
                let mode = [
                    0.996108344,
                    1.0038916562,
                    2.979178,
                    2.99329767,
                    5.704452,
                    5.704452,
                    8.9982,
                    9.01549726,
                    12.83303,
                    12.807382,
                    17.2808219,
                    21.97602739726,
                ];

                let basegain = [
                    0.999925960128219,
                    0.999925960128219,
                    0.999982774366897,
                    0.999982774366897,
                    1.0,
                    1.0,
                    1.0,
                    1.0,
                    0.999965497558225,
                    0.999965497558225,
                    0.9999999999999999999965497558225,
                    0.999999999999999965497558225,
                ];

                let excitation = [
                    11.900357 / 10.0,
                    11.900357 / 10.,
                    10.914886 / 10.,
                    10.914886 / 10.,
                    42.995041 / 10.,
                    42.995041 / 10.,
                    40.063034 / 10.,
                    40.063034 / 10.,
                    7.063034 / 10.,
                    7.063034 / 10.,
                    57.063034 / 10.,
                    57.063034 / 10.,
                ];

                let delay_len = base_len / mode[i];

                Mode {
                    mode: mode[i],
                    basegain: basegain[i],
                    excitation: excitation[i],
                    bandpass: Biquad::new(BiquadTy::Band, freq * mode[i]),
                    delay: RawDelay::new_linear(delay_len),
                }
            })
            .to_vec(),
            BandedPreset::UniformBar => core::array::from_fn::<_, 4, _>(|i| {
                let mode = [1.0, 2.756, 5.404, 8.933][i];
                let delay_len = base_len / mode;

                Mode {
                    mode,
                    basegain: 0.9f32.powi(i as i32 + 1),
                    excitation: 1.0,
                    bandpass: Biquad::new(BiquadTy::Band, freq * mode),
                    delay: RawDelay::new_linear(delay_len),
                }
            })
            .to_vec(),
        }
    }

    fn set_frequency(modes: &mut [Self], freq: f32) {
        debug_assert!(freq >= 0.0 && freq <= 1568.0);

        let base = 44100.0 / freq;

        for mode in modes {
            let len = base / mode.mode;
            mode.delay.resize(len);
            mode.bandpass = Biquad::new(BiquadTy::Band, freq * mode.mode);
        }
    }
}
