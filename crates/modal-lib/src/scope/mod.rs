use eframe::egui;
use float::FloatScope;
use midi::MidiScope;
use serde::{Deserialize, Serialize};

use runtime::{Value, ValueKind};

mod float;
mod midi;

#[allow(clippy::large_enum_variant)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Scope {
    Float(FloatScope),
    Midi(MidiScope),
    Unknown,
}

impl Default for Scope {
    fn default() -> Self {
        Self::new()
    }
}

impl Scope {
    pub fn new() -> Self {
        Scope::Unknown
    }

    pub fn show(&mut self, ui: &mut egui::Ui) {
        match self {
            Scope::Float(fscope) => fscope.show(ui),
            Scope::Midi(mscope) => mscope.show(ui),
            Scope::Unknown => {}
        }
    }

    pub fn feed(&mut self, mut data: Vec<Value>) {
        data.retain(|value| !matches!(value, Value::None | Value::Disconnected));

        if data.is_empty() {
            return;
        }

        let last_ty: ValueKind = data.last().unwrap().into();

        let mut start_at = data.len() - 1;
        loop {
            if start_at == 0 {
                break;
            }

            if ValueKind::from(&data[start_at - 1]) != last_ty {
                break;
            }

            start_at -= 1;
        }

        match (&self, last_ty) {
            (Scope::Midi(_), ValueKind::Midi) => {}
            (_, ValueKind::Midi) => *self = Scope::Midi(MidiScope::new()),

            (Scope::Float(_), ValueKind::Float) => {}
            (_, ValueKind::Float) => *self = Scope::Float(FloatScope::new()),

            _ => {}
        }

        match self {
            Scope::Float(fscope) => fscope.feed(
                data[start_at..]
                    .iter()
                    .map(|value| value.as_float().unwrap()),
            ),
            Scope::Midi(mscope) => mscope.feed(
                data[start_at..]
                    .iter()
                    .map(|value| value.as_midi().unwrap())
                    .map(|(chan, msg)| (chan, *msg)),
            ),
            _ => {}
        }
    }
}
