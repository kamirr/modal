use eframe::egui;
use std::{any::Any, fmt::Debug, sync::Arc};

use dyn_clone::DynClone;

use super::{Output, Value, ValueKind};

pub mod basic;
pub mod effects;
pub mod filters;
pub mod inputs;
pub mod midi;
pub mod noise;

pub trait NodeConfig {
    fn show(&self, ui: &mut egui::Ui, data: &dyn Any);
    fn show_short(&self, _ui: &mut egui::Ui, _data: &dyn Any) {}
}

pub trait InputUi: Send + Sync {
    fn show_always(&self, _ui: &mut egui::Ui, _verbose: bool) {}
    fn show_disconnected(&self, _ui: &mut egui::Ui, _verbose: bool) {}
}

pub struct Input {
    pub kind: ValueKind,
    pub name: String,
    pub default_value: Option<Arc<dyn InputUi>>,
}

impl Input {
    pub fn new<S: Into<String>>(name: S, kind: ValueKind) -> Self {
        Input {
            kind,
            name: name.into(),
            default_value: None,
        }
    }

    pub fn with_default<S: Into<String>, I: InputUi + 'static>(
        name: S,
        kind: ValueKind,
        default_value: &Arc<I>,
    ) -> Self {
        Input {
            name: name.into(),
            kind,
            default_value: Some(Arc::clone(default_value) as Arc<dyn InputUi>),
        }
    }
}

impl Debug for Input {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Input")
            .field("name", &self.name)
            .field(
                "default_value",
                &self.default_value.as_ref().map(|_| "[ommited]"),
            )
            .finish()
    }
}

#[derive(Debug)]
pub enum NodeEvent {
    RecalcInputs(Vec<Input>),
}

#[typetag::serde(tag = "__ty")]
pub trait Node: DynClone + Debug + Send {
    fn feed(&mut self, _data: &[Value]) -> Vec<NodeEvent> {
        Default::default()
    }

    fn read(&self, _out: &mut [Value]) {}

    fn config(&self) -> Option<Arc<dyn NodeConfig>> {
        None
    }

    fn inputs(&self) -> Vec<Input> {
        Vec::default()
    }

    fn output(&self) -> Vec<Output> {
        vec![Output::new("", ValueKind::Float)]
    }
}

pub trait NodeList {
    fn all(&self) -> Vec<(Box<dyn Node>, String)>;
}

pub mod all {
    pub use super::basic::*;
    pub use super::effects::*;
    pub use super::filters::*;
    pub use super::midi::*;
    pub use super::noise::*;
}
