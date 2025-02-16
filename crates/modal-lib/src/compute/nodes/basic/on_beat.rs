use std::sync::Arc;

use serde::{Deserialize, Serialize};

use runtime::{
    node::{Input, Node, NodeEvent},
    ExternInputs, Value,
};

use crate::compute::inputs::beat::BeatInput;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct OnBeat {
    beat: Arc<BeatInput>,
    out: f32,
}

#[typetag::serde]
impl Node for OnBeat {
    fn feed(&mut self, _inputs: &ExternInputs, data: &[Value]) -> Vec<NodeEvent> {
        self.out = self.beat.process(&data[0]).map(|_| 1.0).unwrap_or(0.0);

        Default::default()
    }

    fn read(&self, out: &mut [Value]) {
        out[0] = Value::Float(self.out)
    }

    fn inputs(&self) -> Vec<Input> {
        vec![Input::stateful("beat", &self.beat)]
    }
}

pub fn on_beat() -> Box<dyn Node> {
    Box::new(OnBeat {
        beat: Arc::new(BeatInput::new(true)),
        out: 0.0,
    })
}
