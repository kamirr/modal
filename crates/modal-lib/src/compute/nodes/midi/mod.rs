use super::NodeList;
use runtime::node::Node;

pub mod fluidlite;
pub mod midi_cc;
pub mod one_note;
pub mod source;

pub struct Midi;

impl NodeList for Midi {
    fn all(&self) -> Vec<(Box<dyn Node>, String, Vec<String>)> {
        vec![
            (
                fluidlite::fluidlite(),
                "Fluidlite Synth".into(),
                vec!["Midi".into()],
            ),
            (midi_cc::midi_cc(), "Midi CC".into(), vec!["Midi".into()]),
            (
                one_note::one_note(),
                "One Note Instrument".into(),
                vec!["Midi".into()],
            ),
            (source::midi_in(), "Midi In".into(), vec!["Midi".into()]),
        ]
    }
}
