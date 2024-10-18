use atomic_enum::atomic_enum;
use serde::{Deserialize, Serialize};

use crate::{
    compute::{node::inputs::percentage::PercentageInput, Value, ValueKind},
    serde_atomic_enum,
    util::enum_combo_box,
};

use crate::node::{Input, Node, NodeConfig, NodeEvent};

use std::{
    any::Any,
    sync::{atomic::Ordering, Arc},
};

#[atomic_enum]
#[derive(PartialEq, derive_more::Display, strum::EnumIter)]
enum IirTy {
    Lpf,
    Hpf,
}

serde_atomic_enum!(AtomicIirTy);

#[derive(Debug, Serialize, Deserialize)]
struct IirConfig {
    filt_ty: AtomicIirTy,
}

impl IirConfig {
    fn new(filt_ty: IirTy) -> Self {
        IirConfig {
            filt_ty: AtomicIirTy::new(filt_ty),
        }
    }
}

impl NodeConfig for IirConfig {
    fn show(&self, ui: &mut eframe::egui::Ui, _data: &dyn Any) {
        let mut filt_ty = self.filt_ty.load(Ordering::Acquire);

        enum_combo_box(ui, &mut filt_ty);

        self.filt_ty.store(filt_ty, Ordering::Release);
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct Iir {
    config: Arc<IirConfig>,
    decay: Arc<PercentageInput>,
    prev_y: f32,
}

impl Iir {
    fn new() -> Self {
        Iir {
            config: Arc::new(IirConfig::new(IirTy::Lpf)),
            decay: Arc::new(PercentageInput::new(0.01)),
            prev_y: 0.0,
        }
    }

    fn next(&mut self, input: f32, decay: &Value) {
        let a = 1.0 - self.decay.get_f32(decay);
        let b = 1.0 - a;
        let new_y = match self.config.filt_ty.load(Ordering::Relaxed) {
            IirTy::Lpf => b * input + a * self.prev_y,
            IirTy::Hpf => b * input - a * self.prev_y,
        };

        self.prev_y = new_y;
    }
}

#[typetag::serde]
impl Node for Iir {
    fn feed(&mut self, data: &[Value]) -> Vec<NodeEvent> {
        self.next(data[0].as_float().unwrap_or_default(), &data[1]);

        Default::default()
    }

    fn read(&self, out: &mut [Value]) {
        out[0] = Value::Float(self.prev_y);
    }

    fn config(&self) -> Option<Arc<dyn NodeConfig>> {
        Some(Arc::clone(&self.config) as Arc<_>)
    }

    fn inputs(&self) -> Vec<Input> {
        vec![
            Input::new("sig", ValueKind::Float),
            Input::stateful("decay", &self.decay),
        ]
    }
}

pub fn iir() -> Box<dyn Node> {
    Box::new(Iir::new())
}
