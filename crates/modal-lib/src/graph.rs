use std::{
    borrow::Cow,
    cell::RefCell,
    collections::HashMap,
    sync::{Arc, Weak},
    time::Instant,
};

use egui_graph_edit::{
    AnyParameterId, DataTypeTrait, Graph, GraphEditorState, InputParamKind, NodeDataTrait, NodeId,
    NodeResponse, NodeTemplateIter, NodeTemplateTrait, UserResponseTrait, WidgetValueTrait,
};

use eframe::egui;
use runtime::{
    node::{InputUi, Node, NodeConfig},
    ValueKind,
};
use serde::{Deserialize, Serialize};

use crate::{
    compute::nodes::{all::source::MidiSourceNew, NodeList},
    scope::Scope,
    util::{self, toggle_button},
};

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct OutputState {
    show_scope: bool,
    pub scope: Option<Scope>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SynthNodeData {
    pub out_states: RefCell<HashMap<String, OutputState>>,
    verbose: RefCell<bool>,
}

impl NodeDataTrait for SynthNodeData {
    type Response = SynthNodeResponse;
    type UserState = SynthGraphState;
    type DataType = SynthDataType;
    type ValueType = SynthValueType;

    fn top_bar_ui(
        &self,
        ui: &mut egui::Ui,
        _node_id: NodeId,
        _graph: &egui_graph_edit::Graph<Self, Self::DataType, Self::ValueType>,
        _user_state: &mut Self::UserState,
    ) -> Vec<egui_graph_edit::NodeResponse<Self::Response, Self>>
    where
        Self::Response: UserResponseTrait,
    {
        if ui
            .add(toggle_button("Full", *self.verbose.borrow()))
            .clicked()
        {
            let mut state = self.verbose.borrow_mut();
            *state = !*state;
        }

        Default::default()
    }

    fn bottom_ui(
        &self,
        ui: &mut egui::Ui,
        node_id: NodeId,
        _graph: &egui_graph_edit::Graph<Self, Self::DataType, Self::ValueType>,
        user_state: &mut Self::UserState,
    ) -> Vec<egui_graph_edit::NodeResponse<Self::Response, Self>>
    where
        Self::Response: UserResponseTrait,
    {
        if !*self.verbose.borrow() {
            if let Some(config) = user_state
                .node_configs
                .get(&node_id)
                .and_then(|wk| wk.upgrade())
            {
                config.show_short(ui, &user_state.ctx);
            }

            return Default::default();
        }

        if let Some(config) = user_state
            .node_configs
            .get(&node_id)
            .and_then(|wk| wk.upgrade())
        {
            config.show(ui, &user_state.ctx);
        }

        Default::default()
    }

    fn output_ui(
        &self,
        ui: &mut egui::Ui,
        node_id: NodeId,
        graph: &Graph<Self, Self::DataType, Self::ValueType>,
        user_state: &mut Self::UserState,
        param_name: &str,
    ) -> Vec<egui_graph_edit::NodeResponse<Self::Response, Self>>
    where
        Self::Response: UserResponseTrait,
    {
        let mut responses = vec![];

        let mut states_guard = self.out_states.borrow_mut();

        let state = states_guard.entry(param_name.to_string()).or_default();

        let port = graph.get_port(node_id, param_name).unwrap();
        let is_playing = user_state.rt_playback == Some((node_id, port));

        let scope_btn = util::toggle_button("👁Scope", state.show_scope);
        let play_btn = util::toggle_button("👂Play", is_playing);

        let horizontal_layout = ui.layout().clone();
        ui.vertical(|ui| {
            let resp = ui.with_layout(horizontal_layout, |ui| {
                ui.label(param_name);
                (ui.add(scope_btn), ui.add(play_btn))
            });

            if resp.inner.0.clicked() {
                state.show_scope = !state.show_scope;
            }

            if resp.inner.1.clicked() {
                if !is_playing {
                    responses.push(NodeResponse::User(SynthNodeResponse::SetRtPlayback(
                        node_id, port,
                    )));
                } else {
                    responses.push(NodeResponse::User(SynthNodeResponse::ClearRtPlayback));
                }
            }

            if state.show_scope && state.scope.is_none() {
                state.scope = Some(Scope::new());
                responses.push(NodeResponse::User(SynthNodeResponse::StartRecording(
                    node_id, port,
                )));
            } else if !state.show_scope && state.scope.is_some() {
                state.scope = None;
                responses.push(NodeResponse::User(SynthNodeResponse::StopRecording(
                    node_id, port,
                )));
            }

            if let Some(scope) = &mut state.scope {
                scope.show(ui);
            }
        });

        responses
    }

    fn separator(
        &self,
        ui: &mut egui::Ui,
        _node_id: NodeId,
        _param_id: AnyParameterId,
        _graph: &Graph<Self, Self::DataType, Self::ValueType>,
        _user_state: &mut Self::UserState,
    ) {
        ui.separator();
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum SynthDataType {
    Float,
    Midi,
    Beat,
}

impl SynthDataType {
    pub fn from_value_kind(ty: runtime::ValueKind) -> Self {
        match ty {
            runtime::ValueKind::Float => SynthDataType::Float,
            runtime::ValueKind::Midi => SynthDataType::Midi,
            runtime::ValueKind::Beat => SynthDataType::Beat,
            _ => unimplemented!("compute kind {ty:?} isn't supported as a graph connection type"),
        }
    }
}

impl DataTypeTrait<SynthGraphState> for SynthDataType {
    fn data_type_color(&self, _user_state: &mut SynthGraphState) -> egui::Color32 {
        match self {
            SynthDataType::Float => egui::Color32::LIGHT_BLUE,
            SynthDataType::Midi => egui::Color32::LIGHT_GREEN,
            SynthDataType::Beat => egui::Color32::LIGHT_RED,
        }
    }

    fn name(&self) -> Cow<str> {
        match self {
            SynthDataType::Float => Cow::Borrowed("signal"),
            SynthDataType::Midi => Cow::Borrowed("MIDI"),
            SynthDataType::Beat => Cow::Borrowed("Beat"),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SynthValueType(pub runtime::Value);

impl SynthValueType {
    pub fn data_type(&self) -> SynthDataType {
        match &self.0 {
            runtime::Value::Float(_) => SynthDataType::Float,
            runtime::Value::Midi { .. } => SynthDataType::Midi,
            runtime::Value::Beat(_) => SynthDataType::Beat,
            _ => unimplemented!(),
        }
    }

    pub fn default_with_type(ty: SynthDataType) -> Self {
        SynthValueType(match ty {
            SynthDataType::Float => runtime::Value::Float(0.0),
            SynthDataType::Midi => runtime::Value::None,
            SynthDataType::Beat => runtime::Value::None,
        })
    }
}

impl Eq for SynthValueType {}

impl Default for SynthValueType {
    fn default() -> Self {
        SynthValueType(runtime::Value::Float(0.0))
    }
}

impl WidgetValueTrait for SynthValueType {
    type Response = SynthNodeResponse;
    type UserState = SynthGraphState;
    type NodeData = SynthNodeData;

    fn value_widget(
        &mut self,
        param_name: &str,
        node_id: NodeId,
        ui: &mut egui::Ui,
        user_state: &mut Self::UserState,
        node_data: &Self::NodeData,
    ) -> Vec<Self::Response> {
        let mut resp = Vec::new();
        let ui_inputs = user_state.node_ui_inputs.get(&node_id).unwrap();

        ui.push_id(param_name, |ui| {
            ui.horizontal(|ui| {
                if let Some(input) = ui_inputs.get(param_name) {
                    input.show_name(ui, param_name);
                    input.show_always(ui, *node_data.verbose.borrow());
                    input.show_disconnected(ui, *node_data.verbose.borrow());

                    if input.needs_deep_update() {
                        resp.push(SynthNodeResponse::UpdateInputType(
                            node_id,
                            param_name.to_owned(),
                            input.value_kind(),
                        ));
                    }
                } else {
                    ui.label(param_name);
                }
            });
        });

        resp
    }

    fn value_widget_connected(
        &mut self,
        param_name: &str,
        node_id: NodeId,
        ui: &mut egui::Ui,
        user_state: &mut Self::UserState,
        node_data: &Self::NodeData,
    ) -> Vec<Self::Response> {
        let mut resp = Vec::new();
        let ui_inputs = user_state.node_ui_inputs.get(&node_id).unwrap();

        ui.push_id(param_name, |ui| {
            ui.horizontal(|ui| {
                ui.label(param_name);
                if let Some(input) = ui_inputs.get(param_name) {
                    input.show_always(ui, *node_data.verbose.borrow());

                    if input.needs_deep_update() {
                        resp.push(SynthNodeResponse::UpdateInputType(
                            node_id,
                            param_name.to_owned(),
                            input.value_kind(),
                        ));
                    }
                }
            });
        });

        resp
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SynthNodeTemplate {
    template: Box<dyn Node>,
    name: String,
    categories: Vec<String>,
}

impl Clone for SynthNodeTemplate {
    fn clone(&self) -> Self {
        SynthNodeTemplate {
            template: dyn_clone::clone_box(&*self.template),
            name: self.name.clone(),
            categories: self.categories.clone(),
        }
    }
}

impl NodeTemplateTrait for SynthNodeTemplate {
    type NodeData = SynthNodeData;
    type DataType = SynthDataType;
    type ValueType = SynthValueType;
    type UserState = SynthGraphState;
    type CategoryType = String;

    fn node_finder_label(&self, _user_state: &mut Self::UserState) -> Cow<str> {
        Cow::Borrowed(&self.name)
    }

    fn node_graph_label(&self, user_state: &mut Self::UserState) -> String {
        self.node_finder_label(user_state).into()
    }

    fn user_data(&self, _user_state: &mut Self::UserState) -> Self::NodeData {
        SynthNodeData {
            out_states: RefCell::new(Default::default()),
            verbose: RefCell::new(true),
        }
    }

    fn build_node(
        &self,
        graph: &mut Graph<Self::NodeData, Self::DataType, Self::ValueType>,
        user_state: &mut Self::UserState,
        node_id: NodeId,
    ) {
        let node: Box<dyn Node> = dyn_clone::clone_box(&*self.template);

        let input_signal = |graph: &mut SynthGraph, name: String, data_type: SynthDataType| {
            graph.add_input_param(
                node_id,
                name,
                data_type,
                SynthValueType::default_with_type(data_type),
                InputParamKind::ConnectionOrConstant,
                true,
            );
        };

        for out in node.output() {
            let out_data_ty = SynthDataType::from_value_kind(out.kind);
            graph.add_output_param(node_id, out.name, out_data_ty);
        }

        let mut ui_inputs = HashMap::new();
        for input in node.inputs() {
            let data_type = SynthDataType::from_value_kind(input.kind);

            input_signal(graph, input.name.clone(), data_type);
            if let Some(default) = input.default_value {
                ui_inputs.insert(input.name, default);
            }
        }

        if let Some(config) = node.config() {
            user_state
                .node_configs
                .insert(node_id, Arc::downgrade(&config));
        }

        user_state.node_ui_inputs.insert(node_id, ui_inputs);
        user_state.nodes.insert(node_id, node);
    }

    fn node_finder_categories(&self, _user_state: &mut Self::UserState) -> Vec<Self::CategoryType> {
        self.categories.clone()
    }
}

pub struct AllSynthNodeTemplates {
    lists: Vec<Box<dyn NodeList>>,
}

impl AllSynthNodeTemplates {
    pub fn new(lists: Vec<Box<dyn NodeList>>) -> Self {
        AllSynthNodeTemplates { lists }
    }
}

impl NodeTemplateIter for &AllSynthNodeTemplates {
    type Item = SynthNodeTemplate;

    fn all_kinds(&self) -> Vec<Self::Item> {
        let mut all = Vec::new();
        for list in &self.lists {
            all.extend(list.all().into_iter().map(|(template, name, categories)| {
                SynthNodeTemplate {
                    template,
                    name,
                    categories,
                }
            }))
        }

        all
    }
}

#[derive(Clone, Debug)]
pub enum SynthNodeResponse {
    SetRtPlayback(NodeId, usize),
    ClearRtPlayback,
    StartRecording(NodeId, usize),
    StopRecording(NodeId, usize),
    UpdateInputType(NodeId, String, ValueKind),
}

impl UserResponseTrait for SynthNodeResponse {}

#[derive(Serialize, Deserialize)]
pub enum MidiCollection {
    Single(Box<dyn MidiSourceNew>),
    List(Vec<Box<dyn MidiSourceNew>>),
}

#[derive(Serialize, Deserialize)]
pub struct SynthCtx {
    pub midi: HashMap<String, MidiCollection>,
    #[serde(skip)]
    #[serde(default = "Instant::now")]
    pub last_updated_jack: Instant,
}

impl Default for SynthCtx {
    fn default() -> Self {
        SynthCtx {
            midi: HashMap::new(),
            last_updated_jack: Instant::now(),
        }
    }
}

#[derive(Default, Serialize, Deserialize)]
pub struct SynthGraphState {
    pub rt_playback: Option<(NodeId, usize)>,
    pub ctx: SynthCtx,

    // node_ui_inputs and node_configs need to be initialized separately
    #[serde(skip)]
    pub node_ui_inputs: HashMap<NodeId, HashMap<String, Arc<dyn InputUi>>>,
    #[serde(skip)]
    pub node_configs: HashMap<NodeId, Weak<dyn NodeConfig>>,

    // this only stores intermediate values, can be skipped during serde
    #[serde(skip)]
    pub nodes: HashMap<NodeId, Box<dyn Node>>,
}

pub type SynthGraph = Graph<SynthNodeData, SynthDataType, SynthValueType>;
pub type SynthEditorState = GraphEditorState<
    SynthNodeData,
    SynthDataType,
    SynthValueType,
    SynthNodeTemplate,
    SynthGraphState,
>;

pub trait SynthGraphExt {
    fn get_port(&self, node_id: NodeId, param_name: &str) -> Option<usize>;
}

impl SynthGraphExt for SynthGraph {
    fn get_port(&self, node_id: NodeId, param_name: &str) -> Option<usize> {
        self.nodes
            .get(node_id)
            .unwrap()
            .outputs
            .iter()
            .enumerate()
            .map(|(i, (name, _))| (i, name))
            .find(|(_i, name)| *name == param_name)
            .map(|(i, _name)| i)
    }
}
