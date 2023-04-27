use std::sync::RwLock;

use eframe::egui;
use midly::MidiMessage;
use serde::{Deserialize, Serialize};

use crate::{graph::SynthCtx, midi::Instrument};

use super::{NodeConfig, NodeList};

pub mod fluidlite;
pub mod freq;
pub mod vel;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Inner {
    name: String,
    track: u32,
    valid: bool,
    instrument: Instrument,
    #[serde(skip)]
    messages: Vec<(u32, MidiMessage)>,
}

#[derive(Serialize, Deserialize)]
pub struct MidiInConf {
    #[serde(with = "crate::util::serde_rwlock")]
    inner: RwLock<Inner>,
    record_raw: bool,
}

impl MidiInConf {
    pub fn new(record_raw: bool) -> Self {
        MidiInConf {
            inner: RwLock::new(Inner {
                name: "".into(),
                track: 0,
                valid: false,
                instrument: Instrument::new(),
                messages: Vec::default(),
            }),
            record_raw,
        }
    }

    pub fn instrument(&self) -> Instrument {
        self.inner.read().unwrap().instrument.clone()
    }

    pub fn messages(&self) -> Vec<(u32, MidiMessage)> {
        std::mem::take(&mut self.inner.write().unwrap().messages)
    }
}

impl Clone for MidiInConf {
    fn clone(&self) -> Self {
        MidiInConf {
            inner: RwLock::new(self.inner.read().unwrap().clone()),
            record_raw: self.record_raw,
        }
    }
}

impl std::fmt::Debug for MidiInConf {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MidiInConf")
            .field("inner", &self.inner.read().unwrap())
            .field("record_raw", &self.record_raw)
            .finish()
    }
}

impl NodeConfig for MidiInConf {
    fn show(&self, ui: &mut eframe::egui::Ui, data: &dyn std::any::Any) {
        let ctx: &SynthCtx = data.downcast_ref().unwrap();
        let mut inner = self.inner.write().unwrap();

        if !ctx.midi.contains_key(&inner.name) {
            inner.name = "".into();
        }

        egui::ComboBox::from_id_source("playback")
            .selected_text(&inner.name)
            .show_ui(ui, |ui| {
                for key in ctx.midi.keys() {
                    ui.selectable_value(&mut inner.name, key.into(), key);
                }
            });

        if let Some(midi) = ctx.midi.get(&inner.name) {
            if self.record_raw {
                midi.raw_messages(true);
            }
            inner.messages.extend(midi.messages());

            if midi.tracks() > 0 {
                inner.track = inner.track.clamp(0, midi.tracks() - 1);
                inner.valid = true;
            } else {
                inner.track = 0;
                inner.valid = false;
            }

            egui::ComboBox::from_label("")
                .selected_text(if inner.valid {
                    format!("Track {}", inner.track + 1)
                } else {
                    "".into()
                })
                .show_ui(ui, |ui| {
                    for k in 0..midi.tracks() {
                        ui.selectable_value(&mut inner.track, k, format!("Track {}", k + 1));
                    }
                });

            if inner.valid {
                inner.instrument = midi.instrument(inner.track).clone();
            }
        }
    }
}

pub struct Midi;

impl NodeList for Midi {
    fn all(&self) -> Vec<(Box<dyn super::Node>, String)> {
        vec![
            (freq::midi_freq(), "Midi Frequency".into()),
            (vel::midi_vel(), "Midi Velocity".into()),
            (fluidlite::fluidlite(), "Fluidlite Synth".into()),
        ]
    }
}
