use super::{Node, NodeList};

pub mod chorus;
pub mod clip;
pub mod glide;
pub mod reverb;

pub struct Effects;

impl NodeList for Effects {
    fn all(&self) -> Vec<(Box<dyn Node>, String)> {
        vec![
            (chorus::chorus(), "Chorus".into()),
            (clip::clip(), "Clip".into()),
            (glide::glide(), "Glide".into()),
            (reverb::reverb(), "Reverb".into()),
        ]
    }
}
