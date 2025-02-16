use std::{collections::VecDeque, sync::Arc};

use serde::{Deserialize, Serialize};

use runtime::{
    node::{Input, Node, NodeEvent},
    ExternInputs, Value, ValueKind,
};

use crate::compute::inputs::{slider::SliderInput, time::TimeInput};

#[derive(Clone, Debug, Serialize, Deserialize)]
struct Reverb {
    delays: [VecDeque<f32>; 4],
    times: [Arc<TimeInput>; 4],
    drywet: Arc<SliderInput>,
    feedback: Arc<SliderInput>,
    out: f32,
}

#[typetag::serde]
impl Node for Reverb {
    fn feed(&mut self, _inputs: &ExternInputs, data: &[Value]) -> Vec<NodeEvent> {
        let sample = data[0].as_float().unwrap_or(0.0);
        let drywet = self.drywet.as_f32(&data[1]);
        let feedback = self.feedback.as_f32(&data[2]);

        let outs: Vec<_> = self.delays.iter().map(|d| *d.back().unwrap()).collect();

        for (k, delay) in self.delays.iter_mut().enumerate() {
            let value = sample
                + feedback
                    * 0.5
                    * match k {
                        0 => outs[0] + outs[1] + outs[2] + outs[3],
                        1 => -outs[0] + outs[1] - outs[2] + outs[3],
                        2 => -outs[0] - outs[1] + outs[2] + outs[3],
                        3 => outs[0] - outs[1] - outs[2] + outs[3],
                        _ => unreachable!(),
                    };

            delay.push_front(value);
            let target_samples = self.times[k].get_samples(&data[3 + k]);

            if delay.len() >= target_samples as usize {
                delay.pop_back();
            }
            if delay.len() > target_samples as usize {
                delay.pop_back();
            }
        }

        let wet = outs.iter().sum::<f32>() / 4.0;

        self.out = sample * drywet + wet * (1.0 - drywet);

        Default::default()
    }

    fn read(&self, out: &mut [Value]) {
        out[0] = Value::Float(self.out)
    }

    fn inputs(&self) -> Vec<Input> {
        vec![
            Input::new("sig", ValueKind::Float),
            Input::stateful("dry/wet", &self.drywet),
            Input::stateful("feedback", &self.feedback),
            Input::stateful("t1", &self.times[0]),
            Input::stateful("t2", &self.times[1]),
            Input::stateful("t3", &self.times[2]),
            Input::stateful("t4", &self.times[3]),
        ]
    }
}

pub fn reverb() -> Box<dyn Node> {
    Box::new(Reverb {
        delays: [delay(1), delay(1), delay(1), delay(1)],
        times: [
            Arc::new(TimeInput::from_ms(17.0)),
            Arc::new(TimeInput::from_ms(23.0)),
            Arc::new(TimeInput::from_ms(53.0)),
            Arc::new(TimeInput::from_ms(127.0)),
        ],
        drywet: Arc::new(SliderInput::new(0.5, 0.0, 1.0)),
        feedback: Arc::new(SliderInput::new(0.5, 0.0, 1.0)),
        out: 0.0,
    })
}

fn delay(ms: usize) -> VecDeque<f32> {
    std::iter::repeat(0.0).take(ms * 44100 / 1000).collect()
}
