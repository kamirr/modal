pub mod add;
pub mod constant;
pub mod delay;
pub mod gain;
pub mod oscillator;

use super::{Node, NodeList};

pub struct Basic;

impl NodeList for Basic {
    fn all(&self) -> Vec<(Box<dyn Node>, String)> {
        vec![
            (add::add(), "Add".into()),
            (constant::constant(), "Constant".into()),
            (delay::delay(), "Delay".into()),
            (gain::gain(), "Gain".into()),
            (oscillator::oscillator(), "Oscillator".into()),
        ]
    }
}
