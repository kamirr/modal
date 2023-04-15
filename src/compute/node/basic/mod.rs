pub mod add;
pub mod constant;
pub mod delay;
pub mod gain;
pub mod oscillator;

use super::{Node, NodeList};

pub struct Basic;

impl NodeList for Basic {
    fn all(&self) -> Vec<(fn() -> Box<dyn Node>, &'static str)> {
        vec![
            (add::add, "Add"),
            (constant::constant, "Constant"),
            (delay::delay, "Delay"),
            (gain::gain, "Gain"),
            (oscillator::oscillator, "Oscillator"),
        ]
    }
}
