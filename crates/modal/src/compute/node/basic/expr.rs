use std::{
    any::Any,
    fmt::Debug,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
};

use math_jit::{Compiler, Library, Program};
use serde::{Deserialize, Serialize};

use crate::compute::{
    node::{inputs::real::RealInput, Input, Node, NodeConfig, NodeEvent},
    Output, Value, ValueKind,
};

#[derive(Debug, Serialize, Deserialize)]
pub struct ExpressionConfig {
    enabled: [AtomicBool; 8],
    edit: Mutex<String>,
    new_formula: AtomicBool,
    ready: Mutex<String>,
}

impl ExpressionConfig {
    fn new() -> Self {
        let default = "sin(x + y)";
        ExpressionConfig {
            enabled: [
                AtomicBool::new(true),
                AtomicBool::new(true),
                AtomicBool::new(false),
                AtomicBool::new(false),
                AtomicBool::new(false),
                AtomicBool::new(false),
                AtomicBool::new(false),
                AtomicBool::new(false),
            ],
            edit: Mutex::new(String::from(default)),
            new_formula: AtomicBool::new(true),
            ready: Mutex::new(String::from(default)),
        }
    }
}

impl NodeConfig for ExpressionConfig {
    fn show(&self, ui: &mut eframe::egui::Ui, _data: &dyn Any) {
        let mut enabled = self.enabled.each_ref().map(|b| b.load(Ordering::Relaxed));

        ui.vertical(|ui| {
            let mut text = self.edit.lock().unwrap();
            let clicked_elsewhere = ui.text_edit_singleline(&mut *text).clicked_elsewhere();
            if ui.button("Submit").lost_focus() || clicked_elsewhere {
                let mut ready_lock = self.ready.lock().unwrap();
                if &*ready_lock != &*text {
                    *ready_lock = text.clone();
                    self.new_formula.store(true, Ordering::Relaxed);
                }
            }
        });

        ui.collapsing("Inputs", |ui| {
            ui.horizontal(|ui| {
                if ui.button("None").clicked() {
                    enabled.fill(false);
                } else if ui.button("Default").clicked() {
                    enabled = [true, true, false, false, false, false, false, false];
                } else if ui.button("All").clicked() {
                    enabled.fill(true);
                }
            });

            let inputs = ["x", "y", "a", "b", "c", "d", "sig1", "sig2"];
            for (en, name) in enabled.iter_mut().zip(inputs.into_iter()) {
                ui.checkbox(en, name);
            }
        });

        for (en, new_en) in self.enabled.iter().zip(enabled.into_iter()) {
            en.store(new_en, Ordering::Relaxed);
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct ExprInput {
    name: String,
    input: Arc<RealInput>,
    enabled: bool,
}

impl ExprInput {
    fn new() -> Vec<ExprInput> {
        vec![
            ExprInput {
                name: "x".to_string(),
                input: Arc::new(RealInput::new(0.0)),
                enabled: true,
            },
            ExprInput {
                name: "y".to_string(),
                input: Arc::new(RealInput::new(0.0)),
                enabled: true,
            },
            ExprInput {
                name: "a".to_string(),
                input: Arc::new(RealInput::new(0.0)),
                enabled: false,
            },
            ExprInput {
                name: "b".to_string(),
                input: Arc::new(RealInput::new(0.0)),
                enabled: false,
            },
            ExprInput {
                name: "c".to_string(),
                input: Arc::new(RealInput::new(0.0)),
                enabled: false,
            },
            ExprInput {
                name: "d".to_string(),
                input: Arc::new(RealInput::new(0.0)),
                enabled: false,
            },
        ]
    }
}

type Func = fn(f32, f32, f32, f32, f32, f32, &mut f32, &mut f32) -> f32;

#[derive(Serialize, Deserialize)]
pub struct Expression {
    #[serde(skip, default)]
    compiler: Option<Arc<Mutex<Compiler>>>,
    #[serde(skip, default)]
    func: Option<Func>,
    #[serde(skip, default)]
    serialization_loss_tracker: Option<()>,
    last_valid_expr: String,
    config: Arc<ExpressionConfig>,
    inputs: Vec<ExprInput>,
    sig1: bool,
    sig2: bool,
    out: (f32, f32, f32),
}

impl Debug for Expression {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Expression")
            .field("compiler", &"[JIT compiler]")
            .field("config", &self.config)
            .field("inputs", &self.inputs)
            .field("sig1", &self.sig1)
            .field("sig2", &self.sig2)
            .field("func", &self.func.is_some().then(|| "[JIT function]"))
            .field("out", &self.out)
            .finish()
    }
}

impl Clone for Expression {
    fn clone(&self) -> Self {
        Expression {
            compiler: self.compiler.clone(),
            func: None,
            serialization_loss_tracker: self.serialization_loss_tracker,
            last_valid_expr: self.last_valid_expr.clone(),
            config: self.config.clone(),
            inputs: self.inputs.clone(),
            sig1: self.sig1,
            sig2: self.sig2,
            out: self.out,
        }
    }
}

#[typetag::serde]
impl Node for Expression {
    fn feed(&mut self, data: &[Value]) -> Vec<NodeEvent> {
        if self.serialization_loss_tracker.is_none() {
            self.serialization_loss_tracker = Some(());
            *self.config.ready.lock().unwrap() = self.last_valid_expr.clone();
            self.config.new_formula.store(true, Ordering::Relaxed);
        }

        if self.config.new_formula.swap(false, Ordering::Relaxed) {
            let formula = self.config.ready.lock().unwrap().clone();
            println!("formula updated to {formula}");

            let compiler = match self.compiler.take() {
                None => Arc::new(Mutex::new(Compiler::new(&Library::default()).unwrap())),
                Some(compiler) => compiler,
            };

            if let Ok(program) = Program::parse_from_infix(formula.as_str()) {
                let compile_result = compiler.lock().unwrap().compile(&program);

                match compile_result {
                    Ok(fun) => {
                        self.func = Some(fun);
                        self.last_valid_expr = formula;
                    }
                    Err(e) => println!("{e:?}"),
                };
            }
        }

        let mut data_idx = 0;
        let mut vals = [0.0; 6];
        let mut sig1 = 0.0;
        let mut sig2 = 0.0;

        for (v_idx, desc) in self.inputs.iter().enumerate() {
            if !desc.enabled {
                continue;
            }

            vals[v_idx] = desc
                .input
                .get_f32(&data.get(data_idx).unwrap_or(&Value::Float(0.0)));
            data_idx += 1;
        }
        if self.sig1 {
            sig1 = data
                .get(data_idx)
                .and_then(Value::as_float)
                .unwrap_or_default();
            data_idx += 1;
        }
        if self.sig2 {
            sig2 = data
                .get(data_idx)
                .and_then(Value::as_float)
                .unwrap_or_default();
        }

        self.out = match &mut self.func {
            Some(fun) => {
                let ret = fun(
                    vals[0], vals[1], vals[2], vals[3], vals[4], vals[5], &mut sig1, &mut sig2,
                );
                (ret as f32, sig1 as f32, sig2 as f32)
            }
            None => (0.0, 0.0, 0.0),
        };

        let mut recalc = false;
        for (input, en) in self.inputs.iter_mut().zip(self.config.enabled.iter()) {
            let en = en.load(Ordering::Relaxed);
            if input.enabled != en {
                recalc = true;
                input.enabled = en;
            }
        }
        let sig1 = self.config.enabled[6].load(Ordering::Relaxed);
        let sig2 = self.config.enabled[7].load(Ordering::Relaxed);

        if self.sig1 != sig1 {
            recalc = true;
            self.sig1 = sig1;
        }

        if self.sig2 != sig2 {
            recalc = true;
            self.sig2 = sig2;
        }

        if recalc {
            vec![NodeEvent::RecalcInputs(self.inputs())]
        } else {
            vec![]
        }
    }

    fn read(&self, out: &mut [Value]) {
        out[0] = Value::Float(self.out.0);
        out[1] = Value::Float(self.out.1);
        out[2] = Value::Float(self.out.2);
    }

    fn config(&self) -> Option<Arc<dyn NodeConfig>> {
        Some(Arc::clone(&self.config) as Arc<dyn NodeConfig>)
    }

    fn inputs(&self) -> Vec<Input> {
        self.inputs
            .iter()
            .filter_map(|desc| desc.enabled.then(|| (&desc.name, &desc.input)))
            .map(|(name, input)| Input::stateful(name, input))
            .chain(self.sig1.then(|| Input::new("sig1", ValueKind::Float)))
            .chain(self.sig2.then(|| Input::new("sig2", ValueKind::Float)))
            .collect()
    }

    fn output(&self) -> Vec<Output> {
        vec![
            Output::new("f(..)", ValueKind::Float),
            Output::new("sig1", ValueKind::Float),
            Output::new("sig2", ValueKind::Float),
        ]
    }
}

pub fn expression() -> Box<dyn Node> {
    Box::new(Expression {
        compiler: None,
        func: None,
        serialization_loss_tracker: Some(()),
        last_valid_expr: String::new(),
        config: Arc::new(ExpressionConfig::new()),
        inputs: ExprInput::new(),
        sig1: false,
        sig2: false,
        out: (0.0, 0.0, 0.0),
    })
}
