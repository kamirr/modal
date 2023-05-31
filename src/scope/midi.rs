use std::collections::VecDeque;

use eframe::{egui, emath::Align};
use midly::MidiMessage;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MidiScope {
    #[serde(skip)]
    memory: VecDeque<(u8, MidiMessage)>,
    len: usize,
}

fn pretty_midi(msg: &MidiMessage) -> String {
    match msg {
        MidiMessage::NoteOn { key, vel } => format!("on k{} v{}", key.as_int(), vel.as_int()),
        MidiMessage::NoteOff { key, .. } => format!("off k{}", key.as_int()),
        MidiMessage::Controller { controller, value } => {
            format!("ctrl c{} v{}", controller.as_int(), value.as_int())
        }
        MidiMessage::Aftertouch { key, vel } => format!("aftr k{} v{}", key.as_int(), vel.as_int()),
        MidiMessage::ChannelAftertouch { vel } => format!("chan aftr v{}", vel.as_int()),
        MidiMessage::PitchBend { bend } => format!("bend {}", bend.as_f32()),
        MidiMessage::ProgramChange { program } => format!("prog {}", program.as_int()),
    }
}

impl MidiScope {
    pub fn new() -> Self {
        MidiScope {
            memory: VecDeque::new(),
            len: 12,
        }
    }

    pub fn show(&mut self, ui: &mut egui::Ui) {
        let drag = egui::DragValue::new(&mut self.len).clamp_range(0..=120);
        ui.horizontal(|ui| {
            ui.label("memory");
            ui.add(drag);
        });

        for (chan, msg) in &self.memory {
            ui.horizontal(|ui| {
                ui.with_layout(egui::Layout::left_to_right(Align::LEFT), |ui| {
                    ui.label(format!("{chan}"));
                });

                ui.with_layout(egui::Layout::right_to_left(Align::RIGHT), |ui| {
                    ui.label(pretty_midi(msg));
                });
            });
            ui.separator();
        }
    }

    pub fn feed(&mut self, data: impl Iterator<Item = (u8, MidiMessage)>) {
        for entry in data {
            self.memory.push_front(entry);

            while self.memory.len() > self.len {
                self.memory.pop_back();
            }
        }
    }
}
