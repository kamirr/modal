use std::f32::consts::PI;

enum PureWave {
    Sine,
    Triangle { skew: f32 },
    Square { duty: f32 },
}

impl PureWave {
    fn sample(&self, t: f32) -> f32 {
        match self {
            PureWave::Sine => ((t - 0.5) * PI).sin(),
            PureWave::Triangle { skew } => {
                let m = 2.0 + (skew * 8.0).powf(2.0);
                let l = 2.0;

                (if t < l / m {
                    m * t / l
                } else if t < 2.0 * l - l / m {
                    1.0 - m / (m - 1.0) / l * (t - l / m)
                } else {
                    m * (t - 2.0 * l) / l
                }) * 2.0
                    - 1.0
            }
            PureWave::Square { duty } => {
                if t < *duty * 2.0 {
                    1.0
                } else {
                    -1.0
                }
            }
        }
    }
}

#[derive(Clone, Copy, Debug)]
enum WaveScaleType {
    SineTriangle,
    TriangleSawtooth,
    SawtoothSquare,
    SquareDuty,
}

impl WaveScaleType {
    fn sample(&self, ratio: f32, t: f32) -> f32 {
        match self {
            WaveScaleType::SineTriangle => {
                PureWave::Sine.sample(t) * (1.0 - ratio)
                    + PureWave::Triangle { skew: 0.0 }.sample(t) * ratio
            }
            WaveScaleType::TriangleSawtooth => PureWave::Triangle { skew: ratio }.sample(t),
            WaveScaleType::SawtoothSquare => {
                PureWave::Triangle { skew: 1.0 }.sample(t) * (1.0 - ratio)
                    + PureWave::Square { duty: 0.5 }.sample(t) * ratio
            }
            WaveScaleType::SquareDuty => PureWave::Square {
                duty: 0.5 + ratio / 2.0,
            }
            .sample(t),
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct WaveScale {
    wave_ty: WaveScaleType,
    ratio: f32,
}

impl WaveScale {
    pub fn new(shape: f32) -> WaveScale {
        let shape = (shape * 0.99 - 0.0001) * 4.0;
        let fract = shape.fract();

        let ty = if shape < 1.0 {
            WaveScaleType::SineTriangle
        } else if shape < 2.0 {
            WaveScaleType::TriangleSawtooth
        } else if shape < 3.0 {
            WaveScaleType::SawtoothSquare
        } else if shape < 9.99 {
            WaveScaleType::SquareDuty
        } else {
            panic!("invalid shape specifier {}", shape / 4.0);
        };

        WaveScale {
            wave_ty: ty,
            ratio: fract,
        }
    }

    pub fn sample(&self, t: f32) -> f32 {
        self.wave_ty.sample(self.ratio, t)
    }
}
