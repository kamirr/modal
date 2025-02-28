use std::{
    any::Any,
    collections::HashMap,
    fmt::Debug,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
    time::{Duration, Instant},
};

use anyhow::Result;

use dyn_clone::{clone_box, DynClone};
use eframe::egui;
use extern_::ExternSourceNew;
use midly::MidiMessage;
use serde::{Deserialize, Serialize};

use crate::graph::{MidiCollection, SynthCtx};
use runtime::{
    node::{Node, NodeConfig, NodeEvent},
    ExternInputs, Output, Value, ValueKind,
};

mod extern_;
pub mod jack;
mod null;
pub mod smf;

pub trait MidiSource: Debug + Send {
    fn try_next(&mut self, extern_inputs: &ExternInputs) -> Option<(u8, MidiMessage)>;
    fn reset(&mut self);
}

#[typetag::serde]
pub trait MidiSourceNew: Debug + DynClone + Send + Sync {
    fn new_src(&self) -> Result<Box<dyn MidiSource>>;
    fn name(&self) -> String;
}

#[derive(Debug, Serialize, Deserialize)]
struct RecoverableMidiSource {
    new: Option<Box<dyn MidiSourceNew>>,
    #[serde(skip)]
    source: Option<Box<dyn MidiSource>>,
}

impl RecoverableMidiSource {
    fn new() -> Self {
        RecoverableMidiSource {
            new: None,
            source: None,
        }
    }

    fn source(&mut self) -> Option<&mut dyn MidiSource> {
        if self.source.is_none() {
            if let Some(constructor) = &self.new {
                self.source = constructor.new_src().ok();
            }
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
            new: self.new.as_ref().map(|new| clone_box(&**new)),
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
    extern_inputs: HashMap<String, u64>,
    #[serde(skip, default = "Instant::now")]
    request_ts: Instant,
}

impl Default for Inner {
    fn default() -> Self {
        Inner {
            replace_new: None,
            name: String::from("Select input"),
            replacing: false,
            extern_inputs: HashMap::default(),
            request_ts: Instant::now(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct MidiInConf {
    inner: Mutex<Inner>,
    request_extern_update: AtomicBool,
}

impl MidiInConf {
    fn new() -> Self {
        MidiInConf {
            inner: Mutex::new(Inner::default()),
            request_extern_update: AtomicBool::new(true),
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

        if inner.request_ts.elapsed() > Duration::from_millis(500) {
            inner.request_ts = Instant::now();
            self.request_extern_update.store(true, Ordering::Relaxed);
        }

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
            for (extern_name, idx) in inner.extern_inputs.clone() {
                ui.menu_button("Extern", |ui| {
                    if ui.button(&extern_name).clicked() {
                        inner.name = extern_name.to_string();
                        inner.replace_new = Some(Box::new(ExternSourceNew {
                            name: extern_name,
                            idx,
                        }));
                        ui.close_menu();
                    }
                });
                any_shown = true;
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
    fn feed(&mut self, inputs: &ExternInputs, _data: &[Value]) -> Vec<NodeEvent> {
        if let Ok(mut conf) = self.conf.inner.try_lock() {
            if let Some(new) = conf.replace_new.take() {
                self.source.new = Some(new);
                self.source.source = None;
            }

            if self.conf.request_extern_update.load(Ordering::Relaxed) {
                conf.extern_inputs = inputs
                    .list()
                    .filter(|&(_name, vk)| (vk == ValueKind::Midi))
                    .map(|(name, _vk)| name.clone())
                    .map(|name| {
                        let idx = inputs.get(&name).unwrap();
                        (name, idx.to_bits())
                    })
                    .collect();
            }
        }

        if self.source.new.is_some() {
            if let Some(source) = self.source.source() {
                self.out = source
                    .try_next(inputs)
                    .map(|(channel, message)| Value::Midi { channel, message })
                    .unwrap_or(Value::None);
            } else {
                self.conf.reset();
                self.source = RecoverableMidiSource::new();
                self.out = Value::Disconnected;
            };
        } else {
            self.out = Value::Disconnected;
        }

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
