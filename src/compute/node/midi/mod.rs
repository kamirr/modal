use super::NodeList;

pub mod freq;
pub mod vel;

pub struct Midi;

impl NodeList for Midi {
    fn all(&self) -> Vec<(Box<dyn super::Node>, String)> {
        vec![
            (freq::midi_freq(), "Midi Frequency".into()),
            (vel::midi_vel(), "Midi Velocity".into()),
        ]
    }
}
