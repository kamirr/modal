use std::collections::VecDeque;

use eframe::{egui, emath::Align};
use midly::{num::u7, MidiMessage};
use serde::{Deserialize, Serialize};

pub fn midi_note_to_str(note: u7) -> &'static str {
    #[rustfmt::skip]
    let notes = [
                           "G♯9", "G9", "F♯9", "F9", "E9", "D♯9", "D9", "C♯9", "C9",
        "B8", "A♯8", "A8", "G♯8", "G8", "F♯8", "F8", "E8", "D♯8", "D8", "C♯8", "C8",
        "B7", "A♯7", "A7", "G♯7", "G7", "F♯7", "F7", "E7", "D♯7", "D7", "C♯7", "C7",
        "B6", "A♯6", "A6", "G♯6", "G6", "F♯6", "F6", "E6", "D♯6", "D6", "C♯6", "C6",
        "B5", "A♯5", "A5", "G♯5", "G5", "F♯5", "F5", "E5", "D♯5", "D5", "C♯5", "C5",
        "B4", "A♯4", "A4", "G♯4", "G4", "F♯4", "F4", "E4", "D♯4", "D4", "C♯4", "C4",
        "B3", "A♯3", "A3", "G♯3", "G3", "F♯3", "F3", "E3", "D♯3", "D3", "C♯3", "C3",
        "B2", "A♯2", "A2", "G♯2", "G2", "F♯2", "F2", "E2", "D♯2", "D2", "C♯2", "C2",
        "B1", "A♯1", "A1", "G♯1", "G1", "F♯1", "F1", "E1", "D♯1", "D1", "C♯1", "C1",
        "B0", "A♯0", "A0", "G♯0", "G0", "F♯0", "F0", "E0", "D♯0", "D0", "C♯0", "C0",
        "B1̠", "A♯1̠", "A1̠", "G♯1̠", "G1̠", "F♯1̠", "F1̠", "E1̠", "D♯1̠", "D1̠",
    ];

    notes[note.as_int() as usize]
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MidiScope {
    #[serde(skip)]
    memory: VecDeque<(u8, MidiMessage)>,
    len: usize,
}

fn pretty_midi(msg: MidiMessage) -> String {
    match msg {
        MidiMessage::NoteOn { key, vel } => {
            format!("on {} v{}", midi_note_to_str(key), vel.as_int())
        }
        MidiMessage::NoteOff { key, .. } => format!("off {}", midi_note_to_str(key)),
        MidiMessage::Controller { controller, value } => {
            format!("ctrl c{} v{}", controller.as_int(), value.as_int())
        }
        MidiMessage::Aftertouch { key, vel } => {
            format!("aftr {} v{}", midi_note_to_str(key), vel.as_int())
        }
        MidiMessage::ChannelAftertouch { vel } => format!("chan aftr v{}", vel.as_int()),
        MidiMessage::PitchBend { bend } => format!("bend {}", bend.as_f32()),
        MidiMessage::ProgramChange { program } => format!("prog {}", program.as_int()),
    }
}

impl Default for MidiScope {
    fn default() -> Self {
        Self::new()
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
        let drag = egui::DragValue::new(&mut self.len).range(0..=120);
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
                    ui.label(pretty_midi(*msg));
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
