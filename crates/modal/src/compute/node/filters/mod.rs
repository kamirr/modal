use super::{Node, NodeList};

pub mod biquad;
pub mod iir;
pub mod one_zero;
pub mod pole_zero;

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
            (
                one_zero::one_zero(),
                "One-Zero Filter".into(),
                vec!["Effect".into(), "Filter".into()],
            ),
            (
                pole_zero::pole_zero(),
                "Pole-Zero Filter".into(),
                vec!["Effect".into(), "Filter".into()],
            ),
        ]
    }
}
