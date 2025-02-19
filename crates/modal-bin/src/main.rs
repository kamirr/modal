mod rodio_out;

use std::{collections::HashMap, fs::File, sync::Arc, time::Instant};

use eframe::egui::{self, Vec2};
use egui_graph_edit::{InputParamKind, NodeId, NodeResponse};
use modal_lib::{
    compute::nodes::all::source::{jack::JackSourceNew, smf::SmfSourceNew, MidiSourceNew},
    graph::MidiCollection,
    remote,
};

use rodio_out::RodioOut;
use runtime::{
    node::{Input, NodeEvent},
    OutputPort, Runtime,
};

use rfd::FileDialog;

use modal_lib::graph::{
    self, OutputState, SynthDataType, SynthEditorState, SynthGraphExt, SynthGraphState,
};
use serde::{Deserialize, Serialize};

fn main() {
    let options = eframe::NativeOptions {
        window_builder: Some(Box::new(|viewport| {
            viewport.with_inner_size(Vec2::new(1600.0, 1200.0))
        })),
        ..Default::default()
    };

    eframe::run_native(
        "Modal",
        options,
        Box::new(|cc| Ok(Box::new(SynthApp::with_context(cc)))),
    )
    .unwrap();
}

#[derive(Serialize, Deserialize)]
pub struct SynthAppState {
    pub rt: Runtime,
    pub mapping: Vec<(NodeId, u64)>,
    pub editor_state: SynthEditorState,
    pub graph_state: SynthGraphState,
}

struct SynthApp {
    state: graph::SynthEditorState,
    user_state: graph::SynthGraphState,
    all_nodes: graph::AllSynthNodeTemplates,
    remote: remote::RuntimeRemote,
    prev_frame: Instant,
}

impl SynthApp {
    fn new(state: Option<SynthAppState>) -> Self {
        pub use modal_lib::compute::nodes::all::*;

        if let Some(SynthAppState {
            rt,
            mapping,
            editor_state,
            mut graph_state,
        }) = state
        {
            for (idx, node) in rt.nodes() {
                let node_id = mapping
                    .iter()
                    .find(|(_, bits)| *bits == idx.to_bits())
                    .unwrap()
                    .0;
                if let Some(config) = node.config() {
                    graph_state
                        .node_configs
                        .insert(node_id, Arc::downgrade(&config));
                }

                graph_state.node_ui_inputs.insert(node_id, HashMap::new());
                let inputs = graph_state.node_ui_inputs.get_mut(&node_id).unwrap();
                for input in node.inputs() {
                    if let Some(default) = input.default_value {
                        inputs.insert(input.name, default);
                    }
                }
            }

            let mut remote =
                remote::RuntimeRemote::from_parts(rt, mapping, Box::new(RodioOut::default()));

            for (node_id, node) in &editor_state.graph.nodes {
                for (param_name, _out_state) in node.user_data.out_states.borrow().iter() {
                    remote.record(
                        node_id,
                        editor_state.graph.get_port(node_id, param_name).unwrap(),
                    );
                }
            }

            remote.play(graph_state.rt_playback);

            SynthApp {
                state: editor_state,
                user_state: graph_state,
                all_nodes: graph::AllSynthNodeTemplates::new(vec![
                    Box::new(Basic),
                    Box::new(Effects),
                    Box::new(Filters),
                    Box::new(Instruments),
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
                    Box::new(Instruments),
                    Box::new(Midi),
                    Box::new(Noise),
                ]),
                remote: remote::RuntimeRemote::start(Box::new(RodioOut::default())),
                prev_frame: Instant::now(),
            }
        }
    }

    fn with_context(cc: &eframe::CreationContext) -> Self {
        cc.egui_ctx
            .all_styles_mut(|style| style.interaction.selectable_labels = false);

        let state: Option<SynthAppState> = cc
            .storage
            .and_then(|storage| eframe::get_value(storage, "synth-app"));

        Self::new(state)
    }
}

impl SynthApp {
    fn recalc_inputs(&mut self, node_id: NodeId, inputs: Vec<Input>) {
        let curr_inputs = self.state.graph.nodes.get(node_id).unwrap().inputs.clone();
        let input_names: Vec<_> = inputs.iter().map(|input| input.name.clone()).collect();

        // remove inputs that exist but aren't in `inputs` arg
        for (name, in_id) in &curr_inputs {
            if !input_names.contains(name) {
                self.state.graph.remove_input_param(*in_id);
            }
        }

        // create inputs that don't exist but are in `inputs` arg
        let ui_inputs = self.user_state.node_ui_inputs.get_mut(&node_id).unwrap();
        for input in inputs {
            if !curr_inputs.iter().any(|(name, _)| name == &input.name) {
                let data_type = graph::SynthDataType::from_value_kind(input.kind);

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

        self.state
            .graph
            .nodes
            .get_mut(node_id)
            .unwrap()
            .inputs
            .sort_by_key(|(name, _id)| {
                input_names
                    .iter()
                    .enumerate()
                    .find(|(_, source_name)| *source_name == name)
                    .unwrap()
                    .0
            });

        // recalculate runtime inputs
        let mut rt_inputs = Vec::new();
        for in_id in self.state.graph.nodes.get(node_id).unwrap().input_ids() {
            let src = self
                .state
                .graph
                .connection(in_id)
                .map(|out| (self.state.graph.get_output(out), out))
                .map(|(out_params, out)| (out_params.node, out))
                .and_then(|(node_id, out)| {
                    self.remote
                        .id_to_index(node_id)
                        .map(|idx| (idx, node_id, out))
                });

            let src = src.map(|(idx, node_id, out_id)| {
                let port = self
                    .state
                    .graph
                    .nodes
                    .get(node_id)
                    .unwrap()
                    .outputs
                    .iter()
                    .enumerate()
                    .find(|(_i, (_name, id))| out_id == *id)
                    .unwrap()
                    .0;

                OutputPort::new(idx, port)
            });

            rt_inputs.push(src);
        }
        self.remote.set_inputs(node_id, rt_inputs);
    }

    fn load_midi(&mut self) {
        if let Some(path) = rfd::FileDialog::new().pick_file() {
            let new = match SmfSourceNew::new(&path) {
                Ok(new) => new,
                Err(e) => {
                    println!("{}", e);
                    return;
                }
            };

            let file_midi = self
                .user_state
                .ctx
                .midi
                .entry("File".to_string())
                .or_insert(MidiCollection::List(Vec::new()));
            let MidiCollection::List(list) = file_midi else {
                unreachable!()
            };
            list.push(Box::new(new));
            list.sort_by_key(|smf| smf.name());
        }
    }

    fn serializable_state(&mut self) -> impl serde::Serialize + '_ {
        let rt_state = self.remote.save_state();
        let editor_state = &self.state;
        let graph_state = &self.user_state;

        #[derive(Serialize)]
        struct SerImpl<'a, 'b> {
            pub rt: Runtime,
            pub mapping: Vec<(NodeId, u64)>,
            pub editor_state: &'a SynthEditorState,
            pub graph_state: &'b SynthGraphState,
        }

        SerImpl {
            rt: rt_state.0,
            mapping: rt_state.1,
            editor_state,
            graph_state,
        }
    }
}

impl eframe::App for SynthApp {
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, "synth-app", &self.serializable_state());
        println!("state saved");
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        self.remote.shutdown();
    }

    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::TopBottomPanel::top("top").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                egui::widgets::global_theme_preference_switch(ui);

                egui::menu::menu_button(ui, "File", |ui| {
                    if ui.button("Save").clicked() {
                        let chosen_path =
                            FileDialog::new().add_filter("json", &["json"]).save_file();

                        let Some(path) = chosen_path else { return };

                        let state = self.serializable_state();
                        match File::create(&path) {
                            Ok(file) => serde_json::to_writer(file, &state).unwrap(),
                            Err(e) => println!("Failed to open file {}: {}", path.display(), e),
                        }
                    }

                    if ui.button("Load").clicked() {
                        let chosen_path =
                            FileDialog::new().add_filter("json", &["json"]).pick_file();

                        let Some(path) = chosen_path else { return };

                        let file = match File::open(&path) {
                            Ok(file) => file,
                            Err(e) => {
                                println!("Failed to open file {}: {}", path.display(), e);
                                return;
                            }
                        };

                        let state = match serde_json::from_reader::<_, SynthAppState>(file) {
                            Ok(state) => state,
                            Err(e) => {
                                println!("Failed to deserialize state {}: {}", path.display(), e);
                                return;
                            }
                        };

                        let _ = std::mem::replace(self, Self::new(Some(state)));
                    }
                });

                if ui.button("Open Midi").clicked() {
                    self.load_midi();
                }

                let fps = 1.0 / self.prev_frame.elapsed().as_secs_f32();
                self.prev_frame = Instant::now();
                ui.label(format!("fps: {fps:.2}"));
            });
        });

        let mut prepend_responses = Vec::new();

        if ctx.input(|state| state.key_pressed(egui::Key::Delete)) {
            prepend_responses.extend(
                self.state
                    .selected_nodes
                    .iter()
                    .copied()
                    .map(NodeResponse::DeleteNodeUi),
            );
        }

        let graph_response = egui::CentralPanel::default()
            .show(ctx, |ui| {
                self.state.draw_graph_editor(
                    ui,
                    &self.all_nodes,
                    &mut self.user_state,
                    prepend_responses,
                )
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

                    let out_node = self.state.graph.nodes.get(out_node_id).unwrap();
                    let out_port = out_node
                        .output_ids()
                        .enumerate()
                        .find(|(_i, id)| output == *id)
                        .unwrap()
                        .0;

                    println!("connect {out_node_id:?}:{out_port} to {in_node_id:?}:{in_idx}");
                    self.remote
                        .connect(out_node_id, out_port, in_node_id, in_idx);
                }
                NodeResponse::User(graph::SynthNodeResponse::SetRtPlayback(id, port)) => {
                    println!("set real-time playback {id:?}:{port}");
                    self.user_state.rt_playback = Some((id, port));
                    self.remote.play(Some((id, port)));
                }
                NodeResponse::User(graph::SynthNodeResponse::ClearRtPlayback) => {
                    println!("disable real-time playback");
                    self.user_state.rt_playback = None;
                    self.remote.play(None);
                }
                NodeResponse::User(graph::SynthNodeResponse::StartRecording(node, port)) => {
                    println!("record {node:?}:{port}");
                    self.remote.record(node, port);
                }
                NodeResponse::User(graph::SynthNodeResponse::StopRecording(node, port)) => {
                    println!("record {node:?}:{port}");
                    self.remote.stop_recording(node, port);
                }
                NodeResponse::User(graph::SynthNodeResponse::UpdateInputType(
                    node,
                    param_name,
                    new_kind,
                )) => {
                    let input_id = self
                        .state
                        .graph
                        .nodes
                        .get_mut(node)
                        .unwrap()
                        .inputs
                        .iter()
                        .find(|(input_name, _input_id)| *input_name == param_name)
                        .unwrap()
                        .1;

                    self.state.graph.update_input_param(
                        input_id,
                        None,
                        Some(SynthDataType::from_value_kind(new_kind)),
                        None,
                        None,
                        None,
                    );
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

        for (out_port, samples) in self.remote.recordings() {
            let Some(node_id) = self.remote.index_to_id(out_port.node) else {
                continue;
            };

            let Some(node) = self.state.graph.nodes.get(node_id) else {
                continue;
            };

            let Some((name, _out_id)) = node.outputs.get(out_port.port) else {
                continue;
            };

            if let Some(OutputState {
                scope: Some(scope), ..
            }) = node.user_data.out_states.borrow_mut().get_mut(name)
            {
                scope.feed(samples.clone());
            }
        }

        let synth_ctx = &mut self.user_state.ctx;
        if synth_ctx.last_updated_jack.elapsed().as_secs_f32() > 2.0 {
            synth_ctx.last_updated_jack = Instant::now();
            let midi_jack = synth_ctx
                .midi
                .entry(String::from("Jack"))
                .or_insert(MidiCollection::List(Vec::new()));
            *midi_jack = MidiCollection::List(
                JackSourceNew::all()
                    .into_iter()
                    .map(|new| Box::new(new) as Box<dyn MidiSourceNew>)
                    .collect(),
            );
        }

        self.remote.wait();
        ctx.request_repaint();
    }
}
