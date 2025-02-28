use std::{f32::consts::PI, sync::Arc};

use rand::Rng;
use runtime::ExternInputs;
use serde::{Deserialize, Serialize};

use crate::compute::inputs::{percentage::PercentageInput, real::RealInput};
use crate::compute::nodes::all::{delay::RawDelay, one_zero::OneZero, pole_zero::RawPoleZero};
use runtime::{
    node::{Input, Node, NodeEvent, NodeExt},
    Value,
};

#[derive(Clone, Debug, Serialize, Deserialize)]
struct ReedTable {
    offset: f32,
    slope: f32,
}

impl ReedTable {
    fn new(offset: f32, slope: f32) -> Self {
        ReedTable { offset, slope }
    }

    fn calculate(&mut self, input: f32) -> f32 {
        let out = self.offset + (self.slope * input);
        out.clamp(-1.0, 1.0)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BlowHole {
    pressure: Arc<RealInput>,
    noise: Arc<RealInput>,
    vibrato: Arc<RealInput>,

    vent_in: Arc<PercentageInput>,
    tonehole_in: Arc<PercentageInput>,

    delays: [RawDelay; 3],
    reed_table: ReedTable,
    tonehole: RawPoleZero,
    vent: RawPoleZero,
    filt: OneZero,
    scatter: f32,
    th_coeff: f32,
    rh_gain: f32,

    out: f32,
}

impl BlowHole {
    pub fn new(lowest_freq: f32) -> Self {
        assert!(lowest_freq >= 0.0);

        let n_delay = 0.5 * 44100.0 / lowest_freq;
        let delays = [
            // reed to the register vent
            RawDelay::new(10),
            // register vent to the tonehole
            RawDelay::new_linear(n_delay + 1.0),
            // tonehole to end of bore
            RawDelay::new(8),
        ];

        let reed_table = ReedTable::new(0.7, -0.3);

        // Calculate the initial tonehole three-port scattering coefficient
        let rb = 0.0075f32; // main bore radius
        let rth = 0.003f32; // tonehole radius
        let scatter = -rth.powi(2) / (rth.powi(2) + 2.0 * rb.powi(2));

        // Calculate tonehole coefficients and set for initially open.
        let te = 1.4 * rth; // effective length of the open hole
        let th_coeff = (te * 2.0 * 44100.0 - 347.23) / (te * 2.0 * 44100.0 + 347.23);
        let tonehole = RawPoleZero::new([1.0, -th_coeff], [th_coeff, -1.0]);

        // Calculate register hole filter coefficients
        let r_rh = 0.0015f32; // register vent radius
        let te = 1.4 * r_rh; // effective length of the open hole
        let xi = 0.0f32; // series resistance term
        let zeta = 347.23 + 2.0 * PI * rb.powi(2) * xi / 1.1769;
        let psi = 2.0 * PI * rb.powi(2) * te / (PI * r_rh.powi(2));
        let rh_coeff = (zeta - 2.0 * 44100.0 * psi) / (zeta + 2.0 * 44100.0 * psi);
        let rh_gain = -347.23 / (zeta + 2.0 * 44100.0 * psi);
        let mut vent = RawPoleZero::new([1.0, rh_coeff], [1.0, 1.0]);
        vent.gain = 0.0;

        let mut this = BlowHole {
            pressure: Arc::new(RealInput::new(0.55)),
            noise: Arc::new(RealInput::new(0.0)),
            vibrato: Arc::new(RealInput::new(0.0)),

            vent_in: Arc::new(PercentageInput::new(50.0)),
            tonehole_in: Arc::new(PercentageInput::new(0.0)),

            delays,
            reed_table,
            tonehole,
            vent,
            filt: OneZero::new(-1.0),
            scatter,
            th_coeff,
            rh_gain,

            out: 0.0,
        };

        this.clear();
        this.set_freq(110.0);

        this
    }

    pub fn clear(&mut self) {
        for delay in &mut self.delays {
            delay.clear();
        }
        self.tonehole.feed(0.0);
        self.vent.feed(0.0);
        self.filt.next(0.0, &Value::Disconnected);
    }

    pub fn set_freq(&mut self, f: f32) {
        let mut delay = (44100.0 / f) * 0.5 - 3.5;
        delay -= self.delays[0].len() + self.delays[2].len();

        self.delays[1].resize(delay);
    }

    pub fn set_vent(&mut self, value: f32) {
        let new = self.rh_gain * value.clamp(0.0, 1.0);
        if new != self.vent.gain {
            self.vent.gain = new;
        }
    }

    pub fn set_tonehole(&mut self, value: f32) {
        let coeff = if value <= 0.0 {
            0.9995
        } else if value >= 1.0 {
            self.th_coeff
        } else {
            0.9995 + (value * (self.th_coeff - 0.9995))
        };

        self.tonehole.a[1] = -coeff;
        self.tonehole.b[0] = coeff;
    }
}

#[typetag::serde]
impl Node for BlowHole {
    fn feed(&mut self, inputs: &ExternInputs, data: &[Value]) -> Vec<NodeEvent> {
        let pressure = {
            let mut raw = self.pressure.get_f32(&data[0]);
            let noise = rand::thread_rng().gen_range(-0.2..0.2); //self.noise.get_f32(&data[1]);
            let vibrato = 0.0; //self.vibrato.get_f32(&data[2]);

            raw += raw * noise;
            raw += raw * vibrato;

            raw
        };

        self.set_vent(self.vent_in.get_f32(&data[3]));
        self.set_tonehole(self.tonehole_in.get_f32(&data[4]));

        // Calculate the differential pressure = reflected - mouthpiece pressures
        let p_diff = self.delays[0].last_out() - pressure;

        // Do two-port junction scattering for register vent
        let pa = pressure + p_diff * self.reed_table.calculate(p_diff);
        let pb = self.delays[1].last_out();

        self.vent.feed(pa + pb);

        self.delays[0].push(self.vent.read() + pb);
        self.out = self.delays[0].last_out();

        // Do three-port junction scattering (under tonehole)
        let pa2 = pa + self.vent.read();
        let pb2 = self.delays[2].last_out();
        let pth = self.tonehole.read();
        let temp = self.scatter * (pa2 + pb2 - 2.0 * pth);

        self.filt
            .feed(inputs, &[Value::Float(pa2 + temp), Value::Disconnected]);
        self.delays[2].push(self.filt.read_f32() * -0.95);
        self.delays[1].push(pb2 + temp);
        self.tonehole.feed(pa2 + pb2 - pth + temp);

        Vec::default()
    }

    fn read(&self, out: &mut [Value]) {
        out[0] = Value::Float(self.out);
    }

    fn inputs(&self) -> Vec<Input> {
        vec![
            Input::stateful("pressure", &self.pressure),
            Input::stateful("noise", &self.noise),
            Input::stateful("vibrato", &self.vibrato),
            Input::stateful("vent", &self.vent_in),
            Input::stateful("tonehole", &self.tonehole_in),
        ]
    }
}
