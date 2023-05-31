use eframe::egui::DragValue;
use serde::{Deserialize, Serialize};
use std::{sync::RwLock, time::Duration};

use crate::compute::{node::InputUi, Value, ValueKind};

#[derive(Clone, Copy, Debug)]
pub struct BeatResponse {
    pub period_secs: f32,
}

#[derive(Debug, Serialize, Deserialize)]
struct Inner {
    num: u16,
    den: u16,
    duration: Duration,
    cnt_in: u16,
    cnt_out: u16,
    last_sync: usize,
    notes: [bool; 24],
    note_select: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BeatInput {
    inner: RwLock<Inner>,
}

impl BeatInput {
    pub fn new(note_select: bool) -> Self {
        BeatInput {
            inner: RwLock::new(Inner {
                num: 1,
                den: 1,
                cnt_in: 0,
                cnt_out: 0,
                duration: Duration::from_secs(60),
                last_sync: 0,
                notes: [true; 24],
                note_select,
            }),
        }
    }

    pub fn process(&self, recv: &Value) -> Option<BeatResponse> {
        let mut inner = self.inner.write().unwrap();

        let scale = (inner.num as f32) / (inner.den as f32);

        if let Some(new_dur) = recv.as_beat() {
            inner.cnt_in += 1;

            if inner.cnt_in >= inner.den {
                inner.cnt_in = 0;
                inner.cnt_out = 0;
                inner.duration = new_dur;
                inner.last_sync = 0;

                if inner.notes[0] {
                    return Some(BeatResponse {
                        period_secs: inner.duration.as_secs_f32() / scale,
                    });
                } else {
                    return None;
                }
            }
        }

        inner.last_sync += 1;

        let elapsed_since_sync = inner.last_sync as f32 / 44100.0;
        let passed =
            elapsed_since_sync / inner.duration.as_secs_f32() * inner.num as f32 / inner.den as f32;
        let passed = passed.floor() as u16;

        let tick = if passed > inner.cnt_out {
            inner.cnt_out = passed;
            inner.notes[(inner.cnt_out as usize) % inner.notes.len()]
        } else {
            false
        };

        if tick {
            Some(BeatResponse {
                period_secs: inner.duration.as_secs_f32() / scale,
            })
        } else {
            None
        }
    }
}

impl InputUi for BeatInput {
    fn value_kind(&self) -> ValueKind {
        ValueKind::Beat
    }

    fn show_always(&self, ui: &mut eframe::egui::Ui, _verbose: bool) {
        let mut inner = self.inner.write().unwrap();

        ui.vertical(|ui| {
            ui.horizontal(|ui| {
                ui.add(DragValue::new(&mut inner.num).clamp_range(1..=24));
                ui.label(":");
                ui.add(DragValue::new(&mut inner.den).clamp_range(1..=24));
            });

            if inner.note_select {
                let step = 8;
                let used_notes = inner.num as usize;
                for k in (0..used_notes).step_by(step) {
                    ui.horizontal(|ui| {
                        for i in k..(k + step).min(used_notes) {
                            ui.checkbox(&mut inner.notes[i], "");
                        }
                    });
                }
            }
        });
    }
}
