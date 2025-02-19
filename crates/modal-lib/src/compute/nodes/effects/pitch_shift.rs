use std::{
    fmt::Debug,
    ops::{Deref, DerefMut},
    sync::Arc,
};

use runtime::{
    node::{Input, Node, NodeEvent},
    ExternInputs, Value, ValueKind,
};
use serde::{Deserialize, Serialize};
use serde_big_array::BigArray;
use signalsmith_stretch::Stretch;

use crate::compute::inputs::real::RealInput;

struct StretchWrapper(Stretch);

impl Debug for StretchWrapper {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("Stretch").field(&"[unknown]").finish()
    }
}

impl Default for StretchWrapper {
    fn default() -> Self {
        StretchWrapper(Stretch::preset_default(1, 44100))
    }
}

impl Clone for StretchWrapper {
    fn clone(&self) -> Self {
        StretchWrapper::default()
    }
}

impl Deref for StretchWrapper {
    type Target = Stretch;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for StretchWrapper {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PitchShift {
    shift: Arc<RealInput>,
    #[serde(skip, default)]
    stretch: StretchWrapper,
    #[serde(with = "BigArray")]
    input: [f32; 441],
    #[serde(with = "BigArray")]
    output: [f32; 441],
    pos: usize,
}

#[typetag::serde]
impl Node for PitchShift {
    fn feed(&mut self, _inputs: &ExternInputs, data: &[Value]) -> Vec<NodeEvent> {
        let sig = data[0].as_float().unwrap_or_default();
        let shift = self.shift.get_f32(&data[1]);
        self.stretch.set_transpose_factor(2f32.powf(shift), None);

        self.input[self.pos] = sig;
        self.pos += 1;

        if self.pos == self.input.len() {
            self.stretch.process(self.input, &mut self.output);
            self.pos = 0;
        }

        Default::default()
    }

    fn read(&self, out: &mut [Value]) {
        out[0] = Value::Float(self.output[self.pos])
    }

    fn inputs(&self) -> Vec<Input> {
        vec![
            Input::new("sig", ValueKind::Float),
            Input::stateful("octaves", &self.shift),
        ]
    }
}

pub fn pitch_shift() -> Box<dyn Node> {
    Box::new(PitchShift {
        shift: Arc::new(RealInput::new(1.0)),
        stretch: StretchWrapper::default(),
        input: [0.0; 441],
        output: [0.0; 441],
        pos: 0,
    })
}
