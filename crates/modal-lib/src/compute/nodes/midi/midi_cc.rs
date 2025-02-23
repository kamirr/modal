use std::{
    any::Any,
    fmt,
    sync::{atomic::Ordering, Arc},
};

use atomic_enum::atomic_enum;
use eframe::egui;
use midly::{num::u7, MidiMessage};
use runtime::{
    node::{Input, Node, NodeConfig, NodeEvent},
    ExternInputs, Value,
};
use serde::{Deserialize, Serialize};

use crate::{compute::inputs::midi::MidiInput, serde_atomic_enum, util::enum_combo_box};

#[atomic_enum]
#[derive(Serialize, Deserialize, PartialEq, Eq, strum::EnumIter)]
pub enum MidiCcKind {
    Modulation,
    FootPedal,
    Volume,
    Expression,
    Effect1,
    Effect2,
    DamperPedal,
    Portamento,
}

serde_atomic_enum!(AtomicMidiCcKind);

impl MidiCcKind {
    pub fn number(self) -> u7 {
        u7::new(match self {
            MidiCcKind::Modulation => 1,
            MidiCcKind::FootPedal => 4,
            MidiCcKind::Volume => 7,
            MidiCcKind::Expression => 11,
            MidiCcKind::Effect1 => 12,
            MidiCcKind::Effect2 => 13,
            MidiCcKind::DamperPedal => 64,
            MidiCcKind::Portamento => 65,
        })
    }

    pub fn binary(self) -> bool {
        match self {
            MidiCcKind::Modulation
            | MidiCcKind::FootPedal
            | MidiCcKind::Volume
            | MidiCcKind::Expression
            | MidiCcKind::Effect1
            | MidiCcKind::Effect2 => false,
            MidiCcKind::DamperPedal | MidiCcKind::Portamento => true,
        }
    }

    pub fn default(self) -> u7 {
        u7::new(match self {
            MidiCcKind::Modulation => 0,
            MidiCcKind::FootPedal => 0,
            MidiCcKind::Volume => 127,
            MidiCcKind::Expression => 127,
            MidiCcKind::Effect1 => 0,
            MidiCcKind::Effect2 => 0,
            MidiCcKind::DamperPedal => 0,
            MidiCcKind::Portamento => 0,
        })
    }
}

impl fmt::Display for MidiCcKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                MidiCcKind::Modulation => "Modulation",
                MidiCcKind::FootPedal => "Foot Pedal",
                MidiCcKind::Volume => "Volume",
                MidiCcKind::Expression => "Expression",
                MidiCcKind::Effect1 => "Effect Controller 1",
                MidiCcKind::Effect2 => "Effect Controller 2",
                MidiCcKind::DamperPedal => "Damper Pedal",
                MidiCcKind::Portamento => "Portamento",
            }
        )
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct MidiCcConfig {
    cc: AtomicMidiCcKind,
}

impl NodeConfig for MidiCcConfig {
    fn show(&self, ui: &mut egui::Ui, _data: &dyn Any) {
        let mut cc = self.cc.load(Ordering::Relaxed);

        ui.horizontal(|ui| {
            enum_combo_box(ui, &mut cc);
        });

        self.cc.store(cc, Ordering::Relaxed);
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MidiCc {
    config: Arc<MidiCcConfig>,
    midi_in: Arc<MidiInput>,
    cc: MidiCcKind,
    value: f32,
}

#[typetag::serde]
impl Node for MidiCc {
    fn feed(&mut self, _inputs: &ExternInputs, data: &[Value]) -> Vec<NodeEvent> {
        let new_cc = self.config.cc.load(Ordering::Relaxed);
        if new_cc != self.cc {
            self.value = new_cc.default().as_int() as f32 / 127.0;
            self.cc = new_cc;
        }

        if let Some((_, msg)) = self.midi_in.pop_msg(&data[0]) {
            match msg {
                MidiMessage::Controller { controller, value } if controller == self.cc.number() => {
                    self.value = value.as_int() as f32 / 127.0;
                }
                _ => {}
            }
        }

        Default::default()
    }

    fn config(&self) -> Option<Arc<dyn NodeConfig>> {
        Some(Arc::clone(&self.config) as Arc<_>)
    }

    fn read(&self, out: &mut [Value]) {
        out[0] = Value::Float(self.value);
    }

    fn inputs(&self) -> Vec<Input> {
        vec![Input::stateful("midi", &self.midi_in)]
    }
}

pub fn midi_cc() -> Box<dyn Node> {
    Box::new(MidiCc {
        config: Arc::new(MidiCcConfig {
            cc: MidiCcKind::FootPedal.into(),
        }),
        midi_in: Arc::new(MidiInput::new()),
        cc: MidiCcKind::FootPedal,
        value: 0.0,
    })
}
