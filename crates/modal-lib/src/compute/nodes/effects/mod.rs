use super::NodeList;
use runtime::node::Node;

pub mod bits;
pub mod chorus;
pub mod clip;
pub mod glide;
pub mod heart;
pub mod pitch_shift;
pub mod reverb;
pub mod reverse_delay;

pub struct Effects;

impl NodeList for Effects {
    fn all(&self) -> Vec<(Box<dyn Node>, String, Vec<String>)> {
        vec![
            (bits::bits(), "Bits".into(), vec!["Effect".into()]),
            (chorus::chorus(), "Chorus".into(), vec!["Effect".into()]),
            (clip::clip(), "Clip".into(), vec!["Effect".into()]),
            (glide::glide(), "Glide".into(), vec!["Effect".into()]),
            (heart::heart(), "Heart".into(), vec!["Effect".into()]),
            (
                pitch_shift::pitch_shift(),
                "Pitch Shift".into(),
                vec!["Effect".into()],
            ),
            (reverb::reverb(), "Reverb".into(), vec!["Effect".into()]),
            (
                reverse_delay::reverse_delay(),
                "Reverse Delay".into(),
                vec!["Effect".into()],
            ),
        ]
    }
}
