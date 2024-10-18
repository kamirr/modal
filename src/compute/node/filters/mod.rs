use super::{Node, NodeList};

mod biquad;
mod iir;

pub struct Filters;

impl NodeList for Filters {
    fn all(&self) -> Vec<(Box<dyn Node>, String, Vec<String>)> {
        vec![
            (
                biquad::biquad(),
                "BiQuad Filter".into(),
                vec!["Effect".into(), "Filter".into()],
            ),
            (
                iir::iir(),
                "IIR Filter Single-Pole".into(),
                vec!["Effect".into(), "Filter".into()],
            ),
        ]
    }
}
