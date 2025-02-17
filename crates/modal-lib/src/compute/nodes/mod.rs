use runtime::node::Node;

pub mod basic;
pub mod effects;
pub mod filters;
pub mod instruments;
pub mod midi;
pub mod noise;

pub trait NodeList: Send + Sync {
    fn all(&self) -> Vec<(Box<dyn Node>, String, Vec<String>)>;
}

pub mod all {
    pub use super::basic::*;
    pub use super::effects::*;
    pub use super::filters::*;
    pub use super::instruments::*;
    pub use super::midi::*;
    pub use super::noise::*;
}
