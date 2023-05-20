use super::{Node, NodeList};

pub mod chorus;
pub mod clip;
pub mod glide;
pub mod reverb;

pub struct Effects;

impl NodeList for Effects {
    fn all(&self) -> Vec<(Box<dyn Node>, String, Vec<String>)> {
        vec![
            (chorus::chorus(), "Chorus".into(), vec!["Effect".into()]),
            (clip::clip(), "Clip".into(), vec!["Effect".into()]),
            (glide::glide(), "Glide".into(), vec!["Effect".into()]),
            (reverb::reverb(), "Reverb".into(), vec!["Effect".into()]),
        ]
    }
}
