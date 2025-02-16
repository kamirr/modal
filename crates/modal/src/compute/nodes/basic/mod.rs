pub mod add;
pub mod adsr;
pub mod any;
pub mod bpm;
pub mod constant;
pub mod convert;
pub mod curve;
pub mod delay;
pub mod difference;
pub mod expr;
pub mod gain;
pub mod gate;
pub mod latch;
pub mod mix;
pub mod mix2;
pub mod on_beat;
pub mod oscillator;
pub mod pulse;
pub mod transform;

use super::NodeList;
use delay::ResizeStrategy;
use runtime::node::Node;

pub struct Basic;

impl NodeList for Basic {
    fn all(&self) -> Vec<(Box<dyn Node>, String, Vec<String>)> {
        vec![
            (add::add(), "Add".into(), vec!["Math".into()]),
            (adsr::adsr(), "Adsr".into(), vec!["Envelope".into()]),
            (any::any(), "Any".into(), vec!["Control".into()]),
            (bpm::bpm(), "BPM".into(), vec!["Control".into()]),
            (
                constant::constant(),
                "Constant".into(),
                vec!["Source".into()],
            ),
            (convert::convert(), "Convert".into(), vec!["Math".into()]),
            (curve::curve(), "Curve".into(), vec!["Source".into()]),
            (
                delay::delay(ResizeStrategy::ZeroFillDrain),
                "Delay".into(),
                vec!["Effect".into()],
            ),
            (
                delay::delay(ResizeStrategy::Resample {
                    freq_div: 44100 / 50,
                }),
                "Resampling Delay".into(),
                vec!["Effect".into()],
            ),
            (
                difference::difference(),
                "Difference".to_string(),
                vec!["Effect".to_string()],
            ),
            (
                expr::expression(),
                "Expression".to_string(),
                vec!["Math".to_string()],
            ),
            (gain::gain(), "Gain".into(), vec!["Effect".into()]),
            (gate::gate(), "Gate".into(), vec!["Control".into()]),
            (latch::latch(), "Latch".into(), vec!["Effect".into()]),
            (mix::mix(), "Mix".into(), vec!["Math".into()]),
            (mix2::mix2(), "Mix 2".into(), vec!["Math".into()]),
            (
                on_beat::on_beat(),
                "On Beat".into(),
                vec!["Control".into(), "Source".into()],
            ),
            (
                oscillator::oscillator(),
                "Oscillator".into(),
                vec!["Source".into()],
            ),
            (pulse::pulse(), "Pulse".into(), vec!["Control".into()]),
            (
                transform::transform(),
                "Transform".into(),
                vec!["Effect".into(), "Math".into()],
            ),
        ]
    }
}
