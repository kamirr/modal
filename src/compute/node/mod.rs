use eframe::egui;
use std::{fmt::Debug, sync::Arc};

use dyn_clone::DynClone;

pub mod basic;
pub mod inputs;
//pub mod basic;
//pub mod compose;
//pub mod filter;
//pub mod noise;

pub trait NodeConfig {
    fn show(&self, ui: &mut egui::Ui);
}

pub trait InputUi: Send + Sync {
    fn show(&self, ui: &mut egui::Ui);
    fn value(&self) -> f32;
}

pub struct Input {
    pub name: String,
    pub default_value: Option<Arc<dyn InputUi>>,
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

pub trait Node: DynClone + Debug + Send {
    fn feed(&mut self, _data: &[Option<f32>]) -> Vec<NodeEvent> {
        Default::default()
    }
    fn read(&self) -> f32 {
        0.0
    }

    fn config(&self) -> Option<Arc<dyn NodeConfig>> {
        None
    }

    fn inputs(&self) -> Vec<Input> {
        Vec::default()
    }
}

pub trait NodeList {
    fn all(&self) -> Vec<(fn() -> Box<dyn Node>, &'static str)>;
}

pub mod all {
    pub use super::basic::*;
    /*pub use super::basic::*;
    pub use super::compose::*;
    pub use super::filter::*;
    pub use super::noise::*;*/
}
