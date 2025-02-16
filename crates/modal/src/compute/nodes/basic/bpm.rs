use std::{sync::Arc, time::Duration};

use serde::{Deserialize, Serialize};

use runtime::{
    node::{Input, Node, NodeEvent},
    Output, Value, ValueKind,
};

use crate::compute::inputs::slider::SliderInput;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Bpm {
    bpm: Arc<SliderInput>,
    out: Value,
    t: usize,
}

#[typetag::serde]
impl Node for Bpm {
    fn feed(&mut self, data: &[Value]) -> Vec<NodeEvent> {
        let bpm = self.bpm.as_f32(&data[0]);

        self.t += 1;
        let mins = (self.t as f32) / 44100.0 / 60.0;
        if mins >= 1.0 / bpm {
            self.t = 0;
            self.out = Value::Beat(Duration::from_secs_f32(60.0 / bpm));
        } else {
            self.out = Value::None;
        }

        Default::default()
    }

    fn read(&self, out: &mut [Value]) {
        out[0] = self.out.clone()
    }

    fn inputs(&self) -> Vec<Input> {
        vec![Input::stateful("BPM", &self.bpm)]
    }

    fn output(&self) -> Vec<Output> {
        vec![Output::new("", ValueKind::Beat)]
    }
}

pub fn bpm() -> Box<dyn Node> {
    Box::new(Bpm {
        bpm: Arc::new(SliderInput::new(60.0, 60.0, 300.0).integral(true)),
        out: Value::None,
        t: 0,
    })
}
