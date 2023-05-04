use std::sync::{atomic::Ordering, Arc};

use atomic_enum::atomic_enum;
use serde::{Deserialize, Serialize};

use crate::{
    compute::{
        node::{
            inputs::{positive::PositiveInput, real::RealInput},
            Input, Node, NodeConfig, NodeEvent,
        },
        Value, ValueKind,
    },
    serde_atomic_enum,
    util::enum_combo_box,
};

#[atomic_enum]
#[derive(PartialEq, Eq, Serialize, Deserialize, derive_more::Display, strum::EnumIter)]
enum GlideType {
    Lerp,
    Exponential,
    Pid,
}

serde_atomic_enum!(AtomicGlideType);

#[derive(Debug, Serialize, Deserialize)]
struct GlideConfig {
    ty: AtomicGlideType,
}

impl NodeConfig for GlideConfig {
    fn show(&self, ui: &mut eframe::egui::Ui, _data: &dyn std::any::Any) {
        let mut ty = self.ty.load(Ordering::Acquire);

        enum_combo_box(ui, &mut ty);

        self.ty.store(ty, Ordering::Release);
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Glide {
    conf: Arc<GlideConfig>,
    lerp_coeff: Arc<RealInput>,
    rate_limit: Arc<PositiveInput>,
    #[serde(with = "crate::util::serde_pid")]
    pid_ctrl: pid::Pid<f32>,
    pid: [Arc<PositiveInput>; 3],
    ty: GlideType,
    out: f32,
}

impl Glide {
    pub fn new() -> Self {
        Glide {
            conf: Arc::new(GlideConfig {
                ty: AtomicGlideType::new(GlideType::Lerp),
            }),
            lerp_coeff: Arc::new(RealInput::new(-3.0)),
            rate_limit: Arc::new(PositiveInput::new(1.0)),
            pid_ctrl: pid::Pid::new(0.0, 0.0),
            pid: [
                Arc::new(PositiveInput::new(15.0)),
                Arc::new(PositiveInput::new(0.1)),
                Arc::new(PositiveInput::new(1.0)),
            ],
            ty: GlideType::Lerp,
            out: 0.0,
        }
    }
}

#[typetag::serde]
impl Node for Glide {
    fn feed(&mut self, data: &[Value]) -> Vec<NodeEvent> {
        let next = data[0].as_float().unwrap_or_default();

        let new_ty = self.conf.ty.load(Ordering::Relaxed);
        let emit_ev = self.ty != new_ty;
        self.ty = new_ty;

        self.out = match self.ty {
            GlideType::Lerp => {
                let lerp_r_exp = self.lerp_coeff.get_f32(data.get(1).unwrap_or(&Value::None));
                let lerp_r = 10f32.powf(lerp_r_exp);
                self.out * (1.0 - lerp_r) + next * lerp_r
            }
            GlideType::Exponential => {
                let rate_coeff = self.rate_limit.get_f32(data.get(1).unwrap_or(&Value::None));
                let rate = rate_coeff * self.out / 44100.0;

                if rate.abs() > (self.out - next).abs() {
                    next
                } else if next > self.out {
                    self.out + rate
                } else {
                    self.out - rate
                }
            }
            GlideType::Pid => {
                let p = self.pid[0].get_f32(data.get(1).unwrap_or(&Value::None));
                let i = self.pid[1].get_f32(data.get(2).unwrap_or(&Value::None));
                let d = self.pid[2].get_f32(data.get(3).unwrap_or(&Value::None));
                let lim = 44100.0 * 10.0;

                self.pid_ctrl.output_limit = 44100.0;
                self.pid_ctrl.p(p, lim).i(i, lim).d(d, lim).setpoint(next);

                self.out + self.pid_ctrl.next_control_output(self.out).output / 44100.0
            }
        };

        if emit_ev {
            vec![NodeEvent::RecalcInputs(self.inputs())]
        } else {
            Default::default()
        }
    }

    fn read(&self) -> Value {
        Value::Float(self.out)
    }

    fn config(&self) -> Option<Arc<dyn NodeConfig>> {
        Some(Arc::clone(&self.conf) as Arc<_>)
    }

    fn inputs(&self) -> Vec<Input> {
        let mut ins = vec![Input::new("sig", ValueKind::Float)];
        match self.ty {
            GlideType::Lerp => ins.push(Input::with_default(
                "lerp-r",
                ValueKind::Float,
                &self.lerp_coeff,
            )),
            GlideType::Exponential => ins.push(Input::with_default(
                "rate",
                ValueKind::Float,
                &self.rate_limit,
            )),
            GlideType::Pid => {
                ins.push(Input::with_default("p", ValueKind::Float, &self.pid[0]));
                ins.push(Input::with_default("i", ValueKind::Float, &self.pid[1]));
                ins.push(Input::with_default("d", ValueKind::Float, &self.pid[2]));
            }
        }

        ins
    }
}

pub fn glide() -> Box<dyn Node> {
    Box::new(Glide::new())
}
