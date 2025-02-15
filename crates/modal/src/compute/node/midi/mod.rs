use super::NodeList;

pub mod fluidlite;
pub mod one_note;
pub mod source;

pub struct Midi;

impl NodeList for Midi {
    fn all(&self) -> Vec<(Box<dyn super::Node>, String, Vec<String>)> {
        vec![
            (
                fluidlite::fluidlite(),
                "Fluidlite Synth".into(),
                vec!["Midi".into()],
            ),
            (
                one_note::one_note(),
                "One Note Instrument".into(),
                vec!["Midi".into()],
            ),
            (source::midi_in(), "Midi In".into(), vec!["Midi".into()]),
        ]
    }
}
