use super::{Node, NodeList};

pub mod glide;

pub struct Effects;

impl NodeList for Effects {
    fn all(&self) -> Vec<(Box<dyn Node>, String)> {
        vec![(glide::glide(), "Glide".into())]
    }
}
