use std::{
    any::Any,
    fmt::Debug,
    sync::{Arc, Mutex},
};

use anyhow::Result;

use dyn_clone::{clone_box, DynClone};
use eframe::egui;
use midly::MidiMessage;
use serde::{Deserialize, Serialize};

use crate::graph::{MidiCollection, SynthCtx};
use runtime::{
    node::{Node, NodeConfig, NodeEvent},
    ExternInputs, Output, Value, ValueKind,
};

use self::null::NullSourceNew;

pub mod jack;
mod null;
pub mod smf;

pub trait MidiSource: Debug + Send {
    fn try_next(&mut self) -> Option<(u8, MidiMessage)>;
    fn reset(&mut self);
}

#[typetag::serde]
pub trait MidiSourceNew: Debug + DynClone + Send + Sync {
    fn new_src(&self) -> Result<Box<dyn MidiSource>>;
    fn name(&self) -> String;
}

#[derive(Debug, Serialize, Deserialize)]
struct RecoverableMidiSource {
    new: Box<dyn MidiSourceNew>,
    #[serde(skip)]
    source: Option<Box<dyn MidiSource>>,
}

impl RecoverableMidiSource {
    fn new() -> Self {
        RecoverableMidiSource {
            new: Box::new(NullSourceNew),
            source: None,
        }
    }

    fn source(&mut self) -> Option<&mut dyn MidiSource> {
        if self.source.is_none() {
            self.source = self.new.new_src().ok();
        }

        match &mut self.source {
            Some(src) => Some(src.as_mut()),
            _ => None,
        }
    }
}

impl Clone for RecoverableMidiSource {
    fn clone(&self) -> Self {
        RecoverableMidiSource {
            new: clone_box(&*self.new),
            source: None,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct Inner {
    #[serde(skip)]
    replace_new: Option<Box<dyn MidiSourceNew>>,
    name: String,
    replacing: bool,
}

impl Default for Inner {
    fn default() -> Self {
        Inner {
            replace_new: None,
            name: String::from("Select input"),
            replacing: false,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct MidiInConf {
    #[serde(with = "crate::util::serde_mutex")]
    inner: Mutex<Inner>,
}

impl MidiInConf {
    fn new() -> Self {
        MidiInConf {
            inner: Mutex::new(Inner::default()),
        }
    }

    fn reset(&self) {
        *self.inner.lock().unwrap() = Inner::default();
    }
}

impl NodeConfig for MidiInConf {
    fn show(&self, ui: &mut egui::Ui, data: &dyn Any) {
        let mut inner = self.inner.lock().unwrap();
        let ctx = data.downcast_ref::<SynthCtx>().unwrap();

        ui.menu_button(inner.name.clone(), |ui| {
            let mut any_shown = false;
            for (kind, collection) in &ctx.midi {
                match collection {
                    MidiCollection::Single(midi_source_new) => {
                        if ui.button(midi_source_new.name()).clicked() {
                            inner.name = midi_source_new.name();
                            inner.replace_new = Some(clone_box(&**midi_source_new));
                            ui.close_menu();
                        }
                        any_shown = true;
                    }
                    MidiCollection::List(midi_source_news) => {
                        if !midi_source_news.is_empty() {
                            ui.menu_button(kind, |ui| {
                                for midi_source_new in midi_source_news {
                                    if ui.button(midi_source_new.name()).clicked() {
                                        inner.name = midi_source_new.name();
                                        inner.replace_new = Some(clone_box(&**midi_source_new));
                                        ui.close_menu();
                                    }
                                }
                            });
                            any_shown = true;
                        }
                    }
                }
            }

            if !any_shown {
                ui.label("No MIDI sources found");
            }
        });
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MidiIn {
    conf: Arc<MidiInConf>,
    source: RecoverableMidiSource,
    out: Value,
}

#[typetag::serde]
impl Node for MidiIn {
    fn feed(&mut self, _inputs: &ExternInputs, _data: &[Value]) -> Vec<NodeEvent> {
        if let Ok(mut conf) = self.conf.inner.try_lock() {
            if let Some(new) = conf.replace_new.take() {
                self.source.new = new;
                self.source.source = None;
            }
        }

        let Some(source) = self.source.source() else {
            self.conf.reset();
            self.source = RecoverableMidiSource::new();
            return Default::default();
        };

        self.out = source
            .try_next()
            .map(|(channel, message)| Value::Midi { channel, message })
            .unwrap_or(Value::None);

        Default::default()
    }

    fn read(&self, out: &mut [Value]) {
        out[0] = self.out.clone()
    }

    fn config(&self) -> Option<Arc<dyn NodeConfig>> {
        Some(Arc::clone(&self.conf) as Arc<_>)
    }

    fn output(&self) -> Vec<Output> {
        vec![Output::new("", ValueKind::Midi)]
    }
}

pub fn midi_in() -> Box<dyn Node> {
    Box::new(MidiIn {
        conf: Arc::new(MidiInConf::new()),
        source: RecoverableMidiSource::new(),
        out: Value::None,
    })
}
