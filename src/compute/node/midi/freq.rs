use std::sync::{
    atomic::{AtomicBool, AtomicU32, Ordering},
    Arc,
};

use atomic_float::AtomicF32;
use eframe::egui;
use serde::{Deserialize, Serialize};

use crate::{
    compute::node::{Node, NodeConfig, NodeEvent},
    graph::SynthCtx,
};

#[derive(Debug, Serialize, Deserialize)]
struct MidiFreqConf {
    track: AtomicU32,
    valid: AtomicBool,
    out: AtomicF32,
}

impl NodeConfig for MidiFreqConf {
    fn show(&self, ui: &mut eframe::egui::Ui, data: &dyn std::any::Any) {
        let ctx: &SynthCtx = data.downcast_ref().unwrap();
        if let Some(midi) = &ctx.midi {
            let mut track = self.track.load(Ordering::Acquire);
            let mut valid = true;

            if midi.tracks() > 0 {
                track = track.clamp(0, midi.tracks() - 1);
            } else {
                track = 0;
                valid = false;
            }

            egui::ComboBox::from_label("")
                .selected_text(if valid {
                    format!("Track {}", track + 1)
                } else {
                    "".into()
                })
                .show_ui(ui, |ui| {
                    for k in 0..midi.tracks() {
                        ui.selectable_value(&mut track, k as u32, format!("Track {}", k + 1));
                    }
                });

            if valid {
                let out = midi.instrument(track).freq();
                self.out.store(out, Ordering::Relaxed);
            }

            self.track.store(track, Ordering::Release);
            self.valid.store(valid, Ordering::Release);
        } else {
            ui.label("Load midi first");
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MidiFreq {
    config: Arc<MidiFreqConf>,
}

impl MidiFreq {
    fn new() -> Self {
        MidiFreq {
            config: Arc::new(MidiFreqConf {
                track: 0.into(),
                valid: false.into(),
                out: 0f32.into(),
            }),
        }
    }
}

#[typetag::serde]
impl Node for MidiFreq {
    fn feed(&mut self, _data: &[Option<f32>]) -> Vec<NodeEvent> {
        Default::default()
    }

    fn read(&self) -> f32 {
        self.config.out.load(Ordering::Relaxed)
    }

    fn config(&self) -> Option<Arc<dyn NodeConfig>> {
        Some(Arc::clone(&self.config) as Arc<_>)
    }
}

pub fn midi_freq() -> Box<dyn Node> {
    Box::new(MidiFreq::new())
}
