use runtime::ExternInputs;
use serde::{Deserialize, Serialize};

use anyhow::Result;

use super::{MidiSource, MidiSourceNew};

#[derive(Debug, Clone)]
struct NullSource;

impl MidiSource for NullSource {
    fn try_next(&mut self, _extern: &ExternInputs) -> Option<(u8, midly::MidiMessage)> {
        None
    }

    fn reset(&mut self) {}
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NullSourceNew;

#[typetag::serde]
impl MidiSourceNew for NullSourceNew {
    fn new_src(&self) -> Result<Box<dyn MidiSource>> {
        Ok(Box::new(NullSource))
    }

    fn name(&self) -> String {
        "Null".into()
    }
}
