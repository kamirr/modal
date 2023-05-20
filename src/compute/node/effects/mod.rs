use super::{Node, NodeList};

pub mod chorus;
pub mod clip;
pub mod glide;

pub struct Effects;

impl NodeList for Effects {
    fn all(&self) -> Vec<(Box<dyn Node>, String)> {
        vec![
            (glide::glide(), "Glide".into()),
            (chorus::chorus(), "Chorus".into()),
            (clip::clip(), "Clip".into()),
        ]
    }
}
