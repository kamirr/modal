use std::{fmt::Debug, sync::Arc};

use midly::MidiMessage;
use serde::{Deserialize, Serialize};

use crate::compute::{
    node::{inputs::midi::MidiInput, Input, Node, NodeEvent},
    Output, Value, ValueKind,
};

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct OneNoteState {
    key: u8,
    vel: u8,
}

impl OneNoteState {
    fn new() -> Self {
        OneNoteState { key: 0, vel: 0 }
    }

    fn update(&mut self, message: &MidiMessage) {
        match message {
            MidiMessage::NoteOn { key, vel } => {
                if vel.as_int() == 0 && key == &self.key {
                    self.key = key.as_int();
                    self.vel = 0;
                } else {
                    self.key = key.as_int();
                    self.vel = vel.as_int();
                }
            }
            MidiMessage::NoteOff { key, .. } => {
                if key == &self.key {
                    self.vel = 0;
                }
            }
            _ => {}
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OneNote {
    midi_in: Arc<MidiInput>,
    state: OneNoteState,
}

impl OneNote {
    pub fn new() -> Self {
        OneNote {
            midi_in: Arc::new(MidiInput::new()),
            state: OneNoteState::new(),
        }
    }
}

#[typetag::serde]
impl Node for OneNote {
    fn feed(&mut self, data: &[Value]) -> Vec<NodeEvent> {
        if let Some((_, msg)) = self.midi_in.pop_msg(&data[0]) {
            self.state.update(&msg);
        }

        Default::default()
    }

    fn read(&self, out: &mut [Value]) {
        out[0] = Value::Float(self.state.key as _);
        out[1] = Value::Float(440.0 * 2f32.powf((self.state.key as f32 - 69.0) / 12.0));
        out[2] = Value::Float(self.state.vel as f32 / 127.0);
    }

    fn inputs(&self) -> Vec<Input> {
        vec![Input::with_default("midi", ValueKind::Midi, &self.midi_in)]
    }

    fn output(&self) -> Vec<Output> {
        vec![
            Output::new("key", ValueKind::Float),
            Output::new("freq", ValueKind::Float),
            Output::new("vel", ValueKind::Float),
        ]
    }
}

pub fn one_note() -> Box<dyn Node> {
    Box::new(OneNote::new())
}
