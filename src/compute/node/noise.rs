use super::{Node, NodeList};

#[derive(Clone, Debug)]
pub struct Uniform;

impl Node for Uniform {
    fn read(&self) -> f32 {
        rand::random()
    }
}

pub struct Noise;

impl NodeList for Noise {
    fn all(&self) -> Vec<(fn() -> Box<dyn Node>, &'static str)> {
        vec![(|| Box::new(Uniform), "Uniform")]
    }
}
