pub mod add;
pub mod adsr;
pub mod any;
pub mod constant;
pub mod convert;
pub mod curve;
pub mod delay;
pub mod gain;
pub mod gate;
pub mod latch;
pub mod mix;
pub mod oscillator;
pub mod pulse;
pub mod transform;

use super::{Node, NodeList};

pub struct Basic;

impl NodeList for Basic {
    fn all(&self) -> Vec<(Box<dyn Node>, String, Vec<String>)> {
        vec![
            (add::add(), "Add".into(), vec!["Math".into()]),
            (adsr::adsr(), "Adsr".into(), vec!["Envelope".into()]),
            (any::any(), "Any".into(), vec!["Control".into()]),
            (
                constant::constant(),
                "Constant".into(),
                vec!["Source".into()],
            ),
            (convert::convert(), "Convert".into(), vec!["Math".into()]),
            (curve::curve(), "Curve".into(), vec!["Source".into()]),
            (delay::delay(), "Delay".into(), vec!["Effect".into()]),
            (gain::gain(), "Gain".into(), vec!["Effect".into()]),
            (gate::gate(), "Gate".into(), vec!["Control".into()]),
            (latch::latch(), "Latch".into(), vec!["Effect".into()]),
            (mix::mix(), "Mix".into(), vec!["Math".into()]),
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
