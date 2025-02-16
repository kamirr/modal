mod banded;
mod blow_hole;
mod twang;

use super::NodeList;
use banded::BandedPreset;
use runtime::node::Node;

pub struct Instruments;

impl NodeList for Instruments {
    fn all(&self) -> Vec<(Box<dyn Node>, String, Vec<String>)> {
        vec![
            (
                Box::new(blow_hole::BlowHole::new(220.0)),
                "Blow Hole".to_string(),
                vec!["Instrument".to_string()],
            ),
            (
                twang::twang(),
                "Twang String".to_string(),
                vec!["Instrument".to_string()],
            ),
            (
                Box::new(banded::Banded::new(BandedPreset::TunedBar)),
                "Tuned Bar".to_string(),
                vec!["Instrument".to_string()],
            ),
            (
                Box::new(banded::Banded::new(BandedPreset::GlassHarmonica)),
                "Glass Harmonica".to_string(),
                vec!["Instrument".to_string()],
            ),
            (
                Box::new(banded::Banded::new(BandedPreset::TibetanPrayerBowl)),
                "Tibetan Prayer Bowl".to_string(),
                vec!["Instrument".to_string()],
            ),
            (
                Box::new(banded::Banded::new(BandedPreset::UniformBar)),
                "Uniform Bar".to_string(),
                vec!["Instrument".to_string()],
            ),
        ]
    }
}
