mod compute;
mod graph;
mod midi;
mod remote;
mod scope;

mod util;

use std::{collections::HashMap, sync::Arc, time::Instant};

use eframe::egui;
use egui_node_graph::{InputParamKind, NodeId, NodeResponse};

use compute::node::{
    self,
    all::source::{smf::SmfSourceNew, MidiSourceNew},
    Input, NodeEvent,
};

use crate::{
    compute::Runtime,
    graph::{SynthEditorState, SynthGraphState},
};

fn main() {
    let options = eframe::NativeOptions {
        initial_window_size: Some(egui::vec2(1600.0, 1200.0)),
        ..Default::default()
    };

    eframe::run_native("Modal", options, Box::new(|cc| Box::new(SynthApp::new(cc)))).unwrap();
}

struct SynthApp {
    state: graph::SynthEditorState,
    user_state: graph::SynthGraphState,
    all_nodes: graph::AllSynthNodeTemplates,
    remote: remote::RuntimeRemote,
    prev_frame: Instant,
}

impl SynthApp {
    fn new(cc: &eframe::CreationContext) -> Self {
        pub use node::all::*;

        let state: Option<(
            (Runtime, Vec<(NodeId, u64)>),
            SynthEditorState,
            SynthGraphState,
        )> = cc
            .storage
            .and_then(|storage| eframe::get_value(storage, "synth-app"));

        if let Some(((rt, mapping), editor, mut user_state)) = state {
            for (idx, node) in rt.nodes() {
                let node_id = mapping
                    .iter()
                    .find(|(_, bits)| *bits == idx.to_bits())
                    .unwrap()
                    .0;
                if let Some(config) = node.config() {
                    user_state
                        .node_configs
                        .insert(node_id, Arc::downgrade(&config));
                }

                user_state.node_ui_inputs.insert(node_id, HashMap::new());
                let inputs = user_state.node_ui_inputs.get_mut(&node_id).unwrap();
                for input in node.inputs() {
                    if let Some(default) = input.default_value {
                        inputs.insert(input.name, default);
                    }
                }
            }

            let mut remote = remote::RuntimeRemote::with_rt_and_mapping(rt, mapping);

            for (node_id, node) in &editor.graph.nodes {
                if node.user_data.scope.borrow().is_some() {
                    remote.record(node_id);
                }
            }

            remote.play(user_state.active_node);

            SynthApp {
                state: editor,
                user_state,
                all_nodes: graph::AllSynthNodeTemplates::new(vec![
                    Box::new(Basic),
                    Box::new(Effects),
                    Box::new(Filters),
                    Box::new(Midi),
                    Box::new(Noise),
                ]),
                remote,
                prev_frame: Instant::now(),
            }
        } else {
            SynthApp {
                state: Default::default(),
                user_state: Default::default(),
                all_nodes: graph::AllSynthNodeTemplates::new(vec![
                    Box::new(Basic),
                    Box::new(Effects),
                    Box::new(Filters),
                    Box::new(Midi),
                    Box::new(Noise),
                ]),
                remote: Default::default(),
                prev_frame: Instant::now(),
            }
        }
    }
}

impl SynthApp {
    fn recalc_inputs(&mut self, node_id: NodeId, inputs: Vec<Input>) {
        let curr_inputs = self.state.graph.nodes.get(node_id).unwrap().inputs.clone();

        // remove inputs that exist but aren't in `inputs` arg
        for (name, in_id) in &curr_inputs {
            if !inputs.iter().any(|input| &input.name == name) {
                self.state.graph.remove_input_param(*in_id);
            }
        }

        // create inputs that don't exist but are in `inputs` arg
        let ui_inputs = self.user_state.node_ui_inputs.get_mut(&node_id).unwrap();
        for input in inputs {
            if !curr_inputs.iter().any(|(name, _)| name == &input.name) {
                let data_type = match input.kind {
                    compute::ValueDiscriminants::Float => graph::SynthDataType::Float,
                    compute::ValueDiscriminants::Midi => graph::SynthDataType::Midi,
                    _ => unimplemented!(),
                };

                self.state.graph.add_input_param(
                    node_id,
                    input.name.clone(),
                    data_type,
                    graph::SynthValueType::default_with_type(data_type),
                    InputParamKind::ConnectionOrConstant,
                    true,
                );
            }

            if let Some(default_value) = input.default_value {
                ui_inputs.insert(input.name, default_value);
            }
        }

        // recalculate runtime inputs
        let mut rt_inputs = Vec::new();
        for in_id in self.state.graph.nodes.get(node_id).unwrap().input_ids() {
            let src = self
                .state
                .graph
                .connection(in_id)
                .map(|out| self.state.graph.get_output(out))
                .map(|out_params| out_params.node)
                .and_then(|node_id| self.remote.id_to_index(node_id));
            rt_inputs.push(src);
        }
        self.remote.set_inputs(node_id, rt_inputs);
    }

    fn load_midi(&mut self) {
        if let Some(path) = rfd::FileDialog::new().pick_file() {
            let new = match SmfSourceNew::new(&path) {
                Ok(new) => new,
                Err(e) => {
                    println!("{}", e.to_string());
                    return;
                }
            };

            self.user_state.ctx.midi_smf.push(new);
            self.user_state.ctx.midi_smf.sort_by_key(|smf| smf.name());
        }
    }
}

impl eframe::App for SynthApp {
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        let rt_state = self.remote.save_state();
        let editor_state = &self.state;
        let user_state = &self.user_state;
        let full_state = (rt_state, editor_state, user_state);
        eframe::set_value(storage, "synth-app", &full_state);
        println!("state saved");
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        self.remote.shutdown();
    }

    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::TopBottomPanel::top("top").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                egui::widgets::global_dark_light_mode_switch(ui);
                if ui.button("Open Midi").clicked() {
                    self.load_midi();
                }

                let fps = 1.0 / self.prev_frame.elapsed().as_secs_f32();
                self.prev_frame = Instant::now();
                ui.label(format!("fps: {fps:.2}"));
            });
        });

        let graph_response = egui::CentralPanel::default()
            .show(ctx, |ui| {
                self.state
                    .draw_graph_editor(ui, &self.all_nodes, &mut self.user_state)
            })
            .inner;
        for node_response in graph_response.node_responses {
            match node_response {
                NodeResponse::CreatedNode(id) => {
                    println!("create node {id:?}");
                    let node = self.user_state.nodes.remove(&id).unwrap();
                    self.remote.insert(id, node);
                }
                NodeResponse::DeleteNodeFull { node_id, .. } => {
                    println!("remove node {node_id:?}");
                    self.remote.remove(node_id);
                }
                NodeResponse::DisconnectEvent { input, .. } => {
                    let Some(in_param) = self.state.graph.try_get_input(input) else {
                        continue;
                    };
                    let in_node_id = in_param.node;
                    let in_node = self.state.graph.nodes.get(in_node_id).unwrap();
                    let in_idx = in_node
                        .input_ids()
                        .enumerate()
                        .find(|(_i, id)| id == &in_param.id)
                        .unwrap()
                        .0;

                    println!("disconnect from {in_node_id:?}:{in_idx:?}");
                    self.remote.disconnect(in_node_id, in_idx);
                }
                NodeResponse::ConnectEventEnded { output, input } => {
                    let out_node_id = self.state.graph.get_output(output).node;
                    let in_param = self.state.graph.get_input(input);
                    let in_node_id = in_param.node;
                    let in_node = self.state.graph.nodes.get(in_node_id).unwrap();
                    let in_idx = in_node
                        .input_ids()
                        .enumerate()
                        .find(|(_i, id)| id == &in_param.id)
                        .unwrap()
                        .0;

                    println!("connect {out_node_id:?} to {in_node_id:?}:{in_idx:?}");
                    self.remote.connect(out_node_id, in_node_id, in_idx);
                }
                NodeResponse::User(graph::SynthNodeResponse::SetActiveNode(id)) => {
                    println!("set active {id:?}");
                    self.user_state.active_node = Some(id);
                    self.remote.play(Some(id));
                }
                NodeResponse::User(graph::SynthNodeResponse::ClearActiveNode) => {
                    println!("unset active");
                    self.user_state.active_node = None;
                    self.remote.play(None);
                }
                NodeResponse::User(graph::SynthNodeResponse::StartRecording(node)) => {
                    println!("record {node:?}");
                    self.remote.record(node);
                }
                NodeResponse::User(graph::SynthNodeResponse::StopRecording(node)) => {
                    println!("record {node:?}");
                    self.remote.stop_recording(node);
                }
                _ => {}
            }
        }

        for (idx, evs) in self.remote.events() {
            let Some(node_id) = self.remote.index_to_id(idx) else {
                continue;
            };

            for ev in evs {
                match ev {
                    NodeEvent::RecalcInputs(inputs) => {
                        self.recalc_inputs(node_id, inputs);
                    }
                }
            }
        }

        for (node_id, samples) in self.remote.recordings() {
            let Some(node) = self.state.graph.nodes.get(node_id) else {
                continue;
            };

            let mut scope_guard = node.user_data.scope.borrow_mut();

            let Some(scope) = &mut *scope_guard else {
                continue;
            };

            scope.feed(samples);
        }

        self.user_state.ctx.update_jack();

        self.remote.wait();
        ctx.request_repaint();
    }
}
