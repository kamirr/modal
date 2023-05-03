use super::NodeList;

pub mod fluidlite;
pub mod source;

pub struct Midi;

impl NodeList for Midi {
    fn all(&self) -> Vec<(Box<dyn super::Node>, String)> {
        vec![
            (fluidlite::fluidlite(), "Fluidlite Synth".into()),
            (source::midi_in(), "Midi In".into()),
        ]
    }
}
