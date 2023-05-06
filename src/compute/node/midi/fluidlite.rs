use std::{collections::VecDeque, fmt::Debug, sync::Arc};

use fluidlite as fl;
use midly::MidiMessage;
use serde::{Deserialize, Serialize};

use crate::compute::{
    node::{inputs::midi::MidiInput, Input, Node, NodeEvent},
    Value, ValueKind,
};

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
    midi_in: Arc<MidiInput>,
    #[serde(skip)]
    synth: MyFluidlite,
    out: f32,
    buf: VecDeque<f32>,
}

impl Fluidlite {
    pub fn new() -> Self {
        Fluidlite {
            midi_in: Arc::new(MidiInput::new()),
            synth: MyFluidlite::default(),
            out: 0.0,
            buf: VecDeque::new(),
        }
    }
}

#[typetag::serde]
impl Node for Fluidlite {
    fn feed(&mut self, data: &[Value]) -> Vec<NodeEvent> {
        match self.midi_in.pop_msg(&data[0]) {
            Some((channel, msg)) => match msg {
                MidiMessage::NoteOn { key, vel } => {
                    let vel = vel.as_int() as u32;
                    let key = key.as_int() as u32;
                    if vel > 0 {
                        self.synth.0.note_on(channel as u32, key, vel).ok();
                    } else {
                        self.synth.0.note_off(channel as u32, key).ok();
                    }
                }
                MidiMessage::NoteOff { key, .. } => {
                    self.synth
                        .0
                        .note_off(channel as u32, key.as_int() as _)
                        .ok();
                }
                MidiMessage::Controller { controller, value } => {
                    self.synth
                        .0
                        .cc(channel as _, controller.as_int() as _, value.as_int() as _)
                        .ok();
                }
                _ => {}
            },
            _ => {}
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

    fn read(&self, out: &mut [Value]) {
        out[0] = Value::Float(self.out)
    }

    fn inputs(&self) -> Vec<Input> {
        vec![Input::with_default("midi", ValueKind::Midi, &self.midi_in)]
    }
}

pub fn fluidlite() -> Box<dyn Node> {
    Box::new(Fluidlite::new())
}
