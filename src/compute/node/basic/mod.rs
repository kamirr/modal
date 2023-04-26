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

use super::{Node, NodeList};

pub struct Basic;

impl NodeList for Basic {
    fn all(&self) -> Vec<(Box<dyn Node>, String)> {
        vec![
            (add::add(), "Add".into()),
            (adsr::adsr(), "Adsr".into()),
            (any::any(), "Any".into()),
            (constant::constant(), "Constant".into()),
            (convert::convert(), "Convert".into()),
            (curve::curve(), "Curve".into()),
            (delay::delay(), "Delay".into()),
            (gain::gain(), "Gain".into()),
            (gate::gate(), "Gate".into()),
            (latch::latch(), "Latch".into()),
            (mix::mix(), "Mix".into()),
            (oscillator::oscillator(), "Oscillator".into()),
            (pulse::pulse(), "Pulse".into()),
        ]
    }
}
