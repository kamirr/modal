use std::{collections::HashMap, fmt::Debug};

use midly::{MetaMessage, MidiMessage};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
struct Beat {
    us: u32,
}

impl Beat {
    fn us(&self) -> u32 {
        self.us
    }

    fn update(&mut self, message: &MetaMessage) {
        if let MetaMessage::Tempo(us) = message {
            self.us = us.as_int();
        }
    }
}

impl Default for Beat {
    fn default() -> Self {
        Beat { us: 666667 }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Controller {
    values: HashMap<u8, u8>,
}

impl Controller {
    pub fn new() -> Self {
        Controller {
            values: HashMap::new(),
        }
    }

    fn update(&mut self, message: &MidiMessage) {
        match message {
            MidiMessage::Controller { controller, value } => {
                self.values.insert(controller.as_int(), value.as_int());
            }
            _ => {}
        }
    }

    pub fn get_control(&self, ctrl: u32) -> Option<f32> {
        self.values.get(&(ctrl as u8)).map(|v| *v as f32 / 127.0)
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct MonoNote {
    key: u8,
    vel: u8,
}

impl MonoNote {
    fn new() -> Self {
        MonoNote { key: 0, vel: 0 }
    }

    fn update(&mut self, message: &MidiMessage) {
        match message {
            MidiMessage::NoteOn { key, vel } => {
                self.key = key.as_int();
                self.vel = vel.as_int();
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

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Instrument {
    pub mono_note: MonoNote,
    pub controller: Controller,
}

impl Instrument {
    pub fn new() -> Self {
        Instrument {
            mono_note: MonoNote::new(),
            controller: Controller::new(),
        }
    }

    fn update(&mut self, message: &MidiMessage) {
        self.mono_note.update(message);
        self.controller.update(message);
    }

    pub fn freq(&self) -> f32 {
        let key = self.mono_note.key as f32;
        440.0 * 2f32.powf((key - 69.0) / 12.0)
    }

    pub fn vel(&self) -> f32 {
        self.mono_note.vel as f32 / 127.0
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum PlaybackStepResponse {
    Idle,
    Finished,
    MadeProgress,
}
