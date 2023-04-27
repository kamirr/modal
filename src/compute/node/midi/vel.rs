use std::sync::Arc;

use serde::{Deserialize, Serialize};

use crate::compute::node::{Node, NodeConfig, NodeEvent};

use super::MidiInConf;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MidiVel {
    config: Arc<MidiInConf>,
}

impl MidiVel {
    fn new() -> Self {
        MidiVel {
            config: Arc::new(MidiInConf::new(false)),
        }
    }
}

#[typetag::serde]
impl Node for MidiVel {
    fn feed(&mut self, _data: &[Option<f32>]) -> Vec<NodeEvent> {
        Default::default()
    }

    fn read(&self) -> f32 {
        self.config.instrument().vel()
    }

    fn config(&self) -> Option<Arc<dyn NodeConfig>> {
        Some(Arc::clone(&self.config) as Arc<_>)
    }
}

pub fn midi_vel() -> Box<dyn Node> {
    Box::new(MidiVel::new())
}
