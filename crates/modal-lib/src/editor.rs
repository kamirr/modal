pub mod tree;

use crate::{
    compute::nodes::all::source::{jack::JackSourceNew, smf::SmfSourceNew, MidiSourceNew},
    graph::MidiCollection,
    remote,
};
use eframe::egui::{self, Button, Color32, TextWrapMode};
use egui_graph_edit::{InputParamKind, NodeId, NodeResponse};
use egui_json_tree::{DefaultExpand, JsonTree, JsonTreeStyle, JsonTreeVisuals, ToggleButtonsState};
use runtime::{
    node::{Input, NodeEvent},
    OutputPort, Runtime, Value,
};
use std::{
    collections::{HashMap, HashSet},
    fs::File,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
    time::Instant,
};
use tree::{EditorIndex, EditorTree};

use crate::graph::{
    self, OutputState, SynthDataType, SynthEditorState, SynthGraphExt, SynthGraphState,
};
use rfd::FileDialog;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct GraphEditorState {
    pub rt: Runtime,
    pub mapping: Vec<(NodeId, u64)>,
    pub editor_state: SynthEditorState,
    pub graph_state: SynthGraphState,
}

impl Clone for GraphEditorState {
    fn clone(&self) -> Self {
        let json = serde_json::to_value(self).unwrap();
        serde_json::from_value(json).unwrap()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UpdateResult {
    Ok,
    TopologyChanged,
}

pub struct SharedEditorData {
    pub editor: Mutex<GraphEditor>,
    pub topology_changed: Arc<AtomicBool>,
}

impl SharedEditorData {
    pub fn new(editor: GraphEditor) -> Self {
        SharedEditorData {
            editor: Mutex::new(editor),
            topology_changed: Arc::new(AtomicBool::new(false)),
        }
    }
}

pub struct ManagedEditor {
    name: String,
    handle: Arc<SharedEditorData>,
}

impl ManagedEditor {
    pub fn new(name: impl Into<String>, handle: Arc<SharedEditorData>) -> Self {
        ManagedEditor {
            name: name.into(),
            handle,
        }
    }
}

pub struct ModalApp {
    // TODO: move away from id_tree
    editors: EditorTree,
    active_editor: EditorIndex,
    prev_frame: Instant,
    pub debug_data: serde_json::Map<String, serde_json::Value>,
    debug_window: bool,
}

impl ModalApp {
    pub fn new(editor: GraphEditor) -> Self {
        let root_editor = ManagedEditor::new("Modal", Arc::new(SharedEditorData::new(editor)));
        let editors = EditorTree::new(root_editor);
        let active_editor = editors.root();
        ModalApp {
            editors,
            active_editor,
            prev_frame: Instant::now(),
            debug_data: serde_json::Map::new(),
            debug_window: false,
        }
    }

    pub fn main_app(&mut self, ctx: &egui::Context) {
        // Mark dangling nodes and all their children as to_remove
        let mut to_remove = HashSet::new();
        for (editor_index, editor_node) in self.editors.iter() {
            if editor_node.parent().is_some() && Arc::strong_count(&editor_node.editor.handle) == 1
            {
                self.editors
                    .traverse_from(editor_index, &mut |remove_id, _| {
                        to_remove.insert(remove_id);
                    });
            }
        }

        // Move active editor up the tree until it is not queued for removal
        while to_remove.contains(&self.active_editor) {
            let parent = self.editors.get(self.active_editor).parent().unwrap();
            self.active_editor = parent;
        }

        for id_to_remove in to_remove {
            self.editors.remove(id_to_remove);
        }

        let active_editor_id = self.active_editor;
        let mut active_editor_guard = self
            .editors
            .get(active_editor_id)
            .editor
            .handle
            .editor
            .lock()
            .unwrap();

        egui::TopBottomPanel::top("top").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                egui::widgets::global_theme_preference_switch(ui);

                egui::menu::menu_button(ui, "File", |ui| {
                    if ui.button("Save").clicked() {
                        let chosen_path =
                            FileDialog::new().add_filter("json", &["json"]).save_file();

                        let Some(path) = chosen_path else { return };

                        let state = active_editor_guard.serializable_state();
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

                        let state = match serde_json::from_reader::<_, GraphEditorState>(file) {
                            Ok(state) => state,
                            Err(e) => {
                                println!("Failed to deserialize state {}: {}", path.display(), e);
                                return;
                            }
                        };

                        active_editor_guard.replace(state);
                    }
                });

                egui::menu::menu_button(ui, "View", |ui| {
                    if ui
                        .add(crate::util::toggle_button("Debug", self.debug_window))
                        .clicked()
                    {
                        self.debug_window = !self.debug_window;
                    }
                });

                ui.menu_button("Assembly", |ui| {
                    fn show(
                        editors: &EditorTree,
                        active_editor: &mut EditorIndex,
                        ui: &mut egui::Ui,
                        node_id: &EditorIndex,
                    ) {
                        let node = editors.get(*node_id);
                        if node.children().is_empty() {
                            let button =
                                Button::new(&node.editor.name).wrap_mode(TextWrapMode::Extend);
                            if ui.add(button).clicked() {
                                *active_editor = *node_id;
                            }
                        } else {
                            let self_clicked = ui
                                .menu_button(&node.editor.name, |ui| {
                                    for child_id in node.children() {
                                        show(editors, &mut *active_editor, &mut *ui, child_id);
                                    }
                                })
                                .response
                                .clicked();

                            if self_clicked {
                                *active_editor = *node_id;
                            }
                        }
                    }

                    show(
                        &self.editors,
                        &mut self.active_editor,
                        &mut *ui,
                        &self.editors.root(),
                    );
                });

                if ui.button("Open Midi").clicked() {
                    active_editor_guard.load_midi();
                }

                let fps = 1.0 / self.prev_frame.elapsed().as_secs_f32();
                self.prev_frame = Instant::now();
                ui.label(format!("fps: {fps:.2}"));
            });
        });

        let result = egui::CentralPanel::default()
            .show(ctx, |ui| {
                let res = active_editor_guard.update(ui);

                if self.debug_window {
                    egui::Window::new("Debug").show(ui.ctx(), |ui| {
                        JsonTree::new(
                            "debug-json-tree",
                            &serde_json::Value::Object(self.debug_data.clone()),
                        )
                        .style(
                            JsonTreeStyle::new()
                                .abbreviate_root(true)
                                .toggle_buttons_state(ToggleButtonsState::VisibleDisabled)
                                .visuals(JsonTreeVisuals {
                                    bool_color: Color32::YELLOW,
                                    ..Default::default()
                                }),
                        )
                        .default_expand(DefaultExpand::All)
                        .show(ui);
                    });
                }

                res
            })
            .inner;

        if result == UpdateResult::TopologyChanged {
            println!("editor emitted TopologyChanged");
            self.editors
                .get(self.active_editor)
                .editor
                .handle
                .topology_changed
                .store(true, Ordering::Relaxed);
        }

        drop(active_editor_guard);

        let mut new_editors = Vec::new();
        let mut visit_editor = None;

        for (editor_node_id, editor_node) in self.editors.iter_mut() {
            let mut editor_guard = editor_node.editor.handle.editor.lock().unwrap();

            if editor_node_id != self.active_editor {
                editor_guard.update_background();
            }

            let mut new_editors_guard = editor_guard.user_state.ctx.new_editors.lock().unwrap();
            for entry in new_editors_guard.drain(..) {
                new_editors.push((editor_node_id, entry));
            }
            drop(new_editors_guard);

            let mut visit_editor_guard = editor_guard.user_state.ctx.visit_editor.lock().unwrap();
            if let Some(editor) = visit_editor_guard.take() {
                visit_editor = Some(editor);
            }
            drop(visit_editor_guard);
        }

        for (parent, editor) in new_editors {
            println!("Adding editor {}", editor.name);
            self.editors.insert(parent, editor);
        }

        if let Some(editor) = visit_editor {
            for (editor_node_id, editor_node) in self.editors.iter() {
                if Arc::ptr_eq(&editor_node.editor.handle, &editor) {
                    self.active_editor = editor_node_id;
                }
            }
        }

        ctx.request_repaint();
    }

    pub fn serializable_state(&mut self) -> impl serde::Serialize + '_ {
        let root = self.editors.root();
        self.editors
            .get(root)
            .editor
            .handle
            .editor
            .lock()
            .unwrap()
            .serializable_state()
    }

    pub fn on_exit(&mut self) {
        for (_, node) in self.editors.iter_mut() {
            node.editor.handle.editor.lock().unwrap().shutdown();
        }
    }
}

pub struct GraphEditor {
    pub user_state: graph::SynthGraphState,
    pub remote: remote::RuntimeRemote,
    state: graph::SynthEditorState,
    all_nodes: graph::AllSynthNodeTemplates,
}

impl GraphEditor {
    pub fn new(remote: remote::RuntimeRemote) -> Self {
        pub use crate::compute::nodes::all::*;

        GraphEditor {
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
            remote,
        }
    }

    pub fn replace(&mut self, state: GraphEditorState) {
        let GraphEditorState {
            rt,
            mapping,
            editor_state,
            mut graph_state,
        } = state;

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

        self.remote.replace_runtime(rt, mapping);

        for (node_id, node) in &editor_state.graph.nodes {
            for (param_name, _out_state) in node.user_data.out_states.borrow().iter() {
                self.remote.record(
                    node_id,
                    editor_state.graph.get_port(node_id, param_name).unwrap(),
                );
            }
        }

        self.remote.play(graph_state.rt_playback);
        self.state = editor_state;
        self.user_state = graph_state;
    }

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

    pub fn get_runtime(&mut self) -> (Runtime, Vec<(NodeId, u64)>) {
        self.remote.save_state()
    }

    pub fn serializable_state(&mut self) -> GraphEditorState {
        let rt_state = self.remote.save_state();

        GraphEditorState {
            rt: rt_state.0,
            mapping: rt_state.1,
            editor_state: serde_json::from_value(serde_json::to_value(&self.state).unwrap())
                .unwrap(),
            graph_state: serde_json::from_value(serde_json::to_value(&self.user_state).unwrap())
                .unwrap(),
        }
    }

    pub fn scope_feed(&mut self, out_port: OutputPort, samples: Vec<Value>) {
        let Some(node_id) = self.remote.index_to_id(out_port.node) else {
            return;
        };

        let Some(node) = self.state.graph.nodes.get(node_id) else {
            return;
        };

        let Some((name, _out_id)) = node.outputs.get(out_port.port) else {
            return;
        };

        if let Some(OutputState {
            scope: Some(scope), ..
        }) = node.user_data.out_states.borrow_mut().get_mut(name)
        {
            scope.feed(samples.clone());
        }
    }

    pub fn update(&mut self, ui: &mut egui::Ui) -> UpdateResult {
        let mut result = UpdateResult::Ok;
        let mut prepend_responses = Vec::new();

        if ui.ctx().input(|state| state.key_pressed(egui::Key::Delete)) {
            prepend_responses.extend(
                self.state
                    .selected_nodes
                    .iter()
                    .copied()
                    .map(NodeResponse::DeleteNodeUi),
            );
        }

        let graph_response = self.state.draw_graph_editor(
            ui,
            &self.all_nodes,
            &mut self.user_state,
            prepend_responses,
        );

        for node_response in graph_response.node_responses {
            match node_response {
                NodeResponse::CreatedNode(id) => {
                    println!("create node {id:?}");
                    let node = self.user_state.nodes.remove(&id).unwrap();
                    self.remote.insert(id, node);
                    result = UpdateResult::TopologyChanged;
                }
                NodeResponse::DeleteNodeFull { node_id, .. } => {
                    println!("remove node {node_id:?}");
                    self.remote.remove(node_id);
                    result = UpdateResult::TopologyChanged;
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
                    result = UpdateResult::TopologyChanged;
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
                    result = UpdateResult::TopologyChanged;
                }
                NodeResponse::User(graph::SynthNodeResponse::SetRtPlayback(id, port)) => {
                    println!("set real-time playback {id:?}:{port}");
                    self.user_state.rt_playback = Some((id, port));
                    self.remote.play(Some((id, port)));
                    result = UpdateResult::TopologyChanged;
                }
                NodeResponse::User(graph::SynthNodeResponse::ClearRtPlayback) => {
                    println!("disable real-time playback");
                    self.user_state.rt_playback = None;
                    self.remote.play(None);
                    result = UpdateResult::TopologyChanged;
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
                    result = UpdateResult::TopologyChanged;
                }
                _ => {}
            }
        }

        self.process_background(&mut result);

        result
    }

    pub fn update_background(&mut self) -> UpdateResult {
        let mut result = UpdateResult::Ok;
        self.process_background(&mut result);
        result
    }

    fn process_background(&mut self, result: &mut UpdateResult) {
        for (_idx, config) in self.user_state.node_configs.iter() {
            if let Some(config) = config.upgrade() {
                config.background_task(&self.user_state.ctx);
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
                        *result = UpdateResult::TopologyChanged;
                    }
                }
            }
        }

        for (out_port, samples) in self.remote.recordings() {
            self.scope_feed(out_port, samples);
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
    }

    pub fn shutdown(&mut self) {
        self.remote.shutdown();
    }
}
