use std::{
    collections::{HashSet, VecDeque},
    sync::Mutex,
};

use eframe::egui;
use serde::{Deserialize, Serialize};

use runtime::{node::InputUi, Value, ValueKind};

#[derive(Debug, Serialize, Deserialize)]
struct Inner {
    channel: Option<u8>,
    on_keys: HashSet<(u8, u8)>,
    #[serde(skip)]
    out: VecDeque<(u8, midly::MidiMessage)>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MidiInput {
    inner: Mutex<Inner>,
}

impl Default for MidiInput {
    fn default() -> Self {
        Self::new()
    }
}

impl MidiInput {
    pub fn new() -> Self {
        MidiInput {
            inner: Mutex::new(Inner {
                channel: None,
                on_keys: HashSet::default(),
                out: VecDeque::default(),
            }),
        }
    }

    // Processes the received value and outputs an optional midi messge.
    //
    // If the input is disconnected, automatically sends note-off msg for all
    // previously enabled notes. Additionally translates note-on with 0 velocity
    // into a note-off event, so consumers don't need to care about this way of
    // disabling notes.
    pub fn pop_msg(&self, recv: &Value) -> Option<(u8, midly::MidiMessage)> {
        let mut inner = self.inner.lock().unwrap();

        if recv.disconnected() {
            for (chan, key) in std::mem::take(&mut inner.on_keys) {
                inner.out.push_back((
                    chan,
                    midly::MidiMessage::NoteOff {
                        key: midly::num::u7::from_int_lossy(key),
                        vel: 127.into(),
                    },
                ));
            }
        } else if let Some((chan, msg)) = recv.as_midi() {
            if let Some(filter) = inner.channel {
                if filter != chan {
                    return inner.out.pop_front();
                }
            }

            match msg {
                midly::MidiMessage::NoteOn { key, vel } => {
                    if vel.as_int() == 0 {
                        inner.out.push_back((
                            chan,
                            midly::MidiMessage::NoteOff {
                                key: *key,
                                vel: 64.into(),
                            },
                        ));

                        inner.on_keys.remove(&(chan, key.as_int()));
                    } else {
                        inner.out.push_back((chan, *msg));
                        inner.on_keys.insert((chan, key.as_int()));
                    }
                }
                midly::MidiMessage::NoteOff { key, .. } => {
                    inner.out.push_back((chan, *msg));
                    inner.on_keys.remove(&(chan, key.as_int()));
                }
                _ => inner.out.push_back((chan, *msg)),
            }
        }

        inner.out.pop_front()
    }
}

impl InputUi for MidiInput {
    fn value_kind(&self) -> ValueKind {
        ValueKind::Midi
    }

    fn show_always(&self, ui: &mut egui::Ui, verbose: bool) {
        if verbose {
            let mut inner = self.inner.lock().unwrap();
            let filter = &mut inner.channel;

            egui::ComboBox::new("", "")
                .selected_text(match filter {
                    Some(n) => format!("channel {n}"),
                    None => "All".into(),
                })
                .show_ui(ui, |ui| {
                    ui.selectable_value(filter, None, "All");
                    for k in 0..12 {
                        ui.selectable_value(filter, Some(k), format!("channel {k}"));
                    }
                });
        }
    }
}
