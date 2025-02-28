use midly::MidiMessage;
use runtime::{ExternInputs, Value};
use serde::{Deserialize, Serialize};
use thunderdome::Index;

use super::{MidiSource, MidiSourceNew};

#[derive(Debug)]
pub struct ExternSource(Index);

impl MidiSource for ExternSource {
    fn try_next(&mut self, inputs: &ExternInputs) -> Option<(u8, MidiMessage)> {
        inputs
            .read(self.0)
            .and_then(|v| {
                if let Value::Midi { channel, message } = v {
                    Some((channel, message))
                } else {
                    None
                }
            })
    }

    fn reset(&mut self) {}
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExternSourceNew {
    pub name: String,
    pub idx: u64,
}

#[typetag::serde]
impl MidiSourceNew for ExternSourceNew {
    fn new_src(&self) -> anyhow::Result<Box<dyn MidiSource>> {
        Ok(Box::new(ExternSource(Index::from_bits(self.idx).unwrap())))
    }

    fn name(&self) -> String {
        self.name.clone()
    }
}
