use std::sync::Arc;

use serde::{Deserialize, Serialize};

use runtime::{
    node::{Input, Node, NodeEvent},
    ExternInputs, Value, ValueKind,
};

use crate::compute::{
    inputs::{
        gate::{GateInput, GateInputState},
        time::TimeInput,
    },
    nodes::all::delay::RawDelay,
};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BufferRepeat {
    gate_in: Arc<GateInput>,
    gate_state: GateInputState,
    gate_prev: bool,
    time_in: Arc<TimeInput>,
    delay_impl: RawDelay,
    cursor: f32,
    out: f32,
}

impl BufferRepeat {
    pub fn new(delay_impl: RawDelay) -> Self {
        BufferRepeat {
            gate_in: Arc::new(GateInput::new(0.5)),
            gate_state: GateInputState::default(),
            gate_prev: false,
            time_in: Arc::new(TimeInput::new(delay_impl.len())),
            delay_impl,
            cursor: 0.0,
            out: 0.0,
        }
    }
}

#[typetag::serde]
impl Node for BufferRepeat {
    fn feed(&mut self, _inputs: &ExternInputs, data: &[Value]) -> Vec<NodeEvent> {
        let sig = data[0].as_float().unwrap_or_default();
        let gate = self.gate_in.gate(&mut self.gate_state, &data[1]);
        let target_len = self.time_in.get_samples(&data[2]);

        let old_len = self.delay_impl.len();
        if old_len != target_len {
            self.delay_impl.resize(target_len);
            self.cursor *= target_len / old_len;
        }

        if gate && !self.gate_prev {
            self.cursor = 0.0;
        }
        self.gate_prev = gate;

        if gate {
            self.out = self.delay_impl.get(self.cursor);
            self.cursor = (self.cursor + 1.0) % self.delay_impl.len();
        } else {
            self.delay_impl.push(sig);
            self.out = sig;
        }

        Default::default()
    }

    fn read(&self, out: &mut [Value]) {
        out[0] = Value::Float(self.out)
    }

    fn inputs(&self) -> Vec<Input> {
        vec![
            Input::new("sig", ValueKind::Float),
            Input::stateful("gate", &self.gate_in),
            Input::stateful("time", &self.time_in),
        ]
    }
}

pub fn buffer_repeat() -> Box<dyn Node> {
    Box::new(BufferRepeat::new(RawDelay::new(4410)))
}
