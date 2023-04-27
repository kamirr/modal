use std::{collections::VecDeque, fmt::Debug, sync::Arc};

use fluidlite as fl;
use midly::MidiMessage;
use serde::{Deserialize, Serialize};

use crate::compute::{
    node::{Node, NodeConfig, NodeEvent},
    Value,
};

use super::MidiInConf;

struct MyFluidlite(fl::Synth);

impl Default for MyFluidlite {
    fn default() -> Self {
        let settings = fl::Settings::new().unwrap();
        let synth = fl::Synth::new(settings).unwrap();
        synth.sfload("./sf_/GuitarA.sf2", true).unwrap();
        MyFluidlite(synth)
    }
}

// Fake, only creates a new instance
impl Clone for MyFluidlite {
    fn clone(&self) -> Self {
        let other = MyFluidlite::default();
        other
    }
}

impl Debug for MyFluidlite {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Synth").field("_", &"ommited").finish()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Fluidlite {
    config: Arc<MidiInConf>,
    #[serde(skip)]
    synth: MyFluidlite,
    out: f32,
    buf: VecDeque<f32>,
}

impl Fluidlite {
    pub fn new() -> Self {
        Fluidlite {
            config: Arc::new(MidiInConf::new(true)),
            synth: MyFluidlite::default(),
            out: 0.0,
            buf: VecDeque::new(),
        }
    }
}

#[typetag::serde]
impl Node for Fluidlite {
    fn feed(&mut self, _data: &[Value]) -> Vec<NodeEvent> {
        for (channel, msg) in self.config.messages() {
            match msg {
                MidiMessage::NoteOn { key, vel } => {
                    self.synth
                        .0
                        .note_on(channel, key.as_int() as _, vel.as_int() as _)
                        .ok();
                }
                MidiMessage::NoteOff { key, .. } => {
                    self.synth.0.note_off(channel, key.as_int() as _).ok();
                }
                _ => {}
            }
        }

        if self.buf.len() == 0 {
            let mut buf = [0.0; 441];
            self.synth.0.write(&mut buf[..]).unwrap();
            // las sample is always 0
            self.buf.extend(&buf[0..440]);
        }

        self.out = self.buf.pop_front().unwrap();

        Default::default()
    }

    fn read(&self) -> Value {
        Value::Float(self.out)
    }

    fn config(&self) -> Option<Arc<dyn NodeConfig>> {
        Some(Arc::clone(&self.config) as Arc<_>)
    }
}

pub fn fluidlite() -> Box<dyn Node> {
    Box::new(Fluidlite::new())
}
