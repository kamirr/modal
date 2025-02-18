use serde::{Deserialize, Serialize};

use runtime::{
    node::{Input, Node, NodeEvent},
    ExternInputs, Value,
};
use thunderdome::Index;

use crate::remote::ExternInput;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ExternInputReader {
    input: ExternInput,
    #[serde(skip)]
    index: Option<Index>,
    out: Value,
}

impl ExternInputReader {
    fn new(input: ExternInput) -> Self {
        ExternInputReader {
            input,
            index: None,
            out: Value::None,
        }
    }
}

#[typetag::serde]
impl Node for ExternInputReader {
    fn feed(&mut self, inputs: &ExternInputs, _data: &[Value]) -> Vec<NodeEvent> {
        let index = *self.index.get_or_insert_with(|| {
            inputs
                .get(&self.input.to_string())
                .expect("Input not defined")
        });

        self.out = inputs.read(index).expect("Input not defined 2");

        Default::default()
    }

    fn read(&self, out: &mut [Value]) {
        out[0] = self.out.clone()
    }

    fn inputs(&self) -> Vec<Input> {
        vec![]
    }
}

pub fn track_audio() -> Box<dyn Node> {
    Box::new(ExternInputReader::new(ExternInput::TrackAudio))
}
