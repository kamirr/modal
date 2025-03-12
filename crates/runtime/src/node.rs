use eframe::egui;
use std::{any::Any, fmt::Debug, sync::Arc};

use dyn_clone::DynClone;

use crate::ExternInputs;

use super::{Output, Value, ValueKind};

pub trait NodeConfig: Send + Sync {
    fn show(&self, ui: &mut egui::Ui, data: &dyn Any);
    fn show_short(&self, _ui: &mut egui::Ui, _data: &dyn Any) {}
    fn background_task(&self, _data: &dyn Any) {}
}

pub trait InputUi: Send + Sync {
    fn value_kind(&self) -> ValueKind;
    fn needs_deep_update(&self) -> bool {
        false
    }
    fn show_name(&self, ui: &mut egui::Ui, name: &str) {
        ui.label(name);
    }
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

    pub fn stateful<S: Into<String>, I: InputUi + 'static>(
        name: S,
        default_value: &Arc<I>,
    ) -> Self {
        let kind = default_value.value_kind();
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
    fn feed(&mut self, _inputs: &ExternInputs, _data: &[Value]) -> Vec<NodeEvent> {
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

pub trait NodeExt {
    fn read_f32(&self) -> f32;
}

impl<T: Node> NodeExt for T {
    fn read_f32(&self) -> f32 {
        let mut buf = [Value::None];
        self.read(&mut buf);
        buf[0].as_float().unwrap()
    }
}
