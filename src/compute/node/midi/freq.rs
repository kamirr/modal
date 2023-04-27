use std::{fmt::Debug, sync::Arc};

use serde::{Deserialize, Serialize};

use crate::compute::{
    node::{Node, NodeConfig, NodeEvent},
    Value,
};

use super::MidiInConf;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MidiFreq {
    config: Arc<MidiInConf>,
}

impl MidiFreq {
    fn new() -> Self {
        MidiFreq {
            config: Arc::new(MidiInConf::new(false)),
        }
    }
}

#[typetag::serde]
impl Node for MidiFreq {
    fn feed(&mut self, _data: &[Value]) -> Vec<NodeEvent> {
        Default::default()
    }

    fn read(&self) -> Value {
        Value::Float(self.config.instrument().freq())
    }

    fn config(&self) -> Option<Arc<dyn NodeConfig>> {
        Some(Arc::clone(&self.config) as Arc<_>)
    }
}

pub fn midi_freq() -> Box<dyn Node> {
    Box::new(MidiFreq::new())
}
