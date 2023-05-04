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

use crate::{
    compute::{
        node::{Node, NodeConfig, NodeEvent},
        Value, ValueKind,
    },
    graph::SynthCtx,
    util,
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

    fn source(&mut self) -> &mut dyn MidiSource {
        if self.source.is_none() {
            self.source = Some(self.new.new_src().unwrap());
        }

        match &mut self.source {
            Some(src) => src.as_mut(),
            _ => unreachable!(),
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

#[derive(Clone, Copy, Debug, derive_more::Display, PartialEq, Eq, Serialize, Deserialize)]
enum SourceKind {
    File,
    Jack,
}

#[derive(Debug, Serialize, Deserialize)]
struct Inner {
    #[serde(skip)]
    replace_new: Option<Box<dyn MidiSourceNew>>,
    replacing: bool,
    source_kind: SourceKind,
}

#[derive(Debug, Serialize, Deserialize)]
struct MidiInConf {
    #[serde(with = "crate::util::serde_mutex")]
    inner: Mutex<Inner>,
}

impl MidiInConf {
    fn new() -> Self {
        MidiInConf {
            inner: Mutex::new(Inner {
                replace_new: None,
                replacing: false,
                source_kind: SourceKind::File,
            }),
        }
    }
}

impl NodeConfig for MidiInConf {
    fn show(&self, ui: &mut egui::Ui, data: &dyn Any) {
        let mut inner = self.inner.lock().unwrap();
        let ctx = data.downcast_ref::<SynthCtx>().unwrap();

        if ui
            .add(util::toggle_button("Change", inner.replacing))
            .clicked()
        {
            inner.replacing = !inner.replacing;
        }

        if inner.replacing {
            egui::Window::new("Choose Midi Source").show(ui.ctx(), |ui| {
                ui.horizontal(|ui| {
                    ui.selectable_value(&mut inner.source_kind, SourceKind::File, "File");
                    ui.selectable_value(&mut inner.source_kind, SourceKind::Jack, "Jack");
                });
                ui.separator();

                egui::ScrollArea::new([false, true]).show(ui, |ui| match inner.source_kind {
                    SourceKind::File => {
                        for new in &ctx.midi_smf {
                            if ui
                                .add(egui::Label::new(new.name()).sense(egui::Sense::click()))
                                .clicked()
                            {
                                inner.replace_new = Some(clone_box(new));
                                inner.replacing = false;
                            }
                        }
                    }
                    SourceKind::Jack => {
                        for new in &ctx.midi_jack {
                            if ui
                                .add(egui::Label::new(new.name()).sense(egui::Sense::click()))
                                .clicked()
                            {
                                inner.replace_new = Some(clone_box(new));
                                inner.replacing = false;
                            }
                        }
                    }
                });
            });
        }
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
    fn feed(&mut self, _data: &[Value]) -> Vec<NodeEvent> {
        if let Ok(mut conf) = self.conf.inner.try_lock() {
            if let Some(new) = conf.replace_new.take() {
                self.source.new = new;
                self.source.source = None;
            }
        }

        self.out = self
            .source
            .source()
            .try_next()
            .map(|(channel, message)| Value::Midi { channel, message })
            .unwrap_or(Value::None);

        Default::default()
    }

    fn read(&self) -> Value {
        self.out.clone()
    }

    fn config(&self) -> Option<Arc<dyn NodeConfig>> {
        Some(Arc::clone(&self.conf) as Arc<_>)
    }

    fn output(&self) -> ValueKind {
        ValueKind::Midi
    }
}

pub fn midi_in() -> Box<dyn Node> {
    Box::new(MidiIn {
        conf: Arc::new(MidiInConf::new()),
        source: RecoverableMidiSource::new(),
        out: Value::None,
    })
}
