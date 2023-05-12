use std::{
    borrow::Cow,
    cell::RefCell,
    collections::HashMap,
    sync::{Arc, Weak},
    time::Instant,
};

use egui_node_graph::{
    DataTypeTrait, Graph, GraphEditorState, InputParamKind, NodeDataTrait, NodeId, NodeResponse,
    NodeTemplateIter, NodeTemplateTrait, UserResponseTrait, WidgetValueTrait,
};

use eframe::{egui, emath::Align};
use serde::{Deserialize, Serialize};

use crate::{
    compute::{
        self,
        node::{
            all::source::{jack::JackSourceNew, smf::SmfSourceNew},
            InputUi, Node, NodeConfig, NodeList,
        },
    },
    scope::Scope,
    util::{self, toggle_button},
};

#[derive(Debug, Serialize, Deserialize)]
pub struct OutputState {
    show_scope: bool,
    pub scope: Option<Scope>,
}

impl Default for OutputState {
    fn default() -> Self {
        OutputState {
            show_scope: false,
            scope: None,
        }
    }
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
        _graph: &egui_node_graph::Graph<Self, Self::DataType, Self::ValueType>,
        _user_state: &mut Self::UserState,
    ) -> Vec<egui_node_graph::NodeResponse<Self::Response, Self>>
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
        _graph: &egui_node_graph::Graph<Self, Self::DataType, Self::ValueType>,
        user_state: &mut Self::UserState,
    ) -> Vec<egui_node_graph::NodeResponse<Self::Response, Self>>
    where
        Self::Response: UserResponseTrait,
    {
        if !*self.verbose.borrow() {
            if let Some(config) = user_state
                .node_configs
                .get(&node_id)
                .and_then(|wk| wk.upgrade())
            {
                config.show_short(ui, &mut user_state.ctx);
            }

            return Default::default();
        }

        if let Some(config) = user_state
            .node_configs
            .get(&node_id)
            .and_then(|wk| wk.upgrade())
        {
            config.show(ui, &mut user_state.ctx);
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
    ) -> Vec<egui_node_graph::NodeResponse<Self::Response, Self>>
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

        let resp = ui.horizontal(|ui| {
            ui.with_layout(egui::Layout::right_to_left(Align::RIGHT), |ui| {
                ui.label(param_name);
                (ui.add(scope_btn), ui.add(play_btn))
            })
        });

        if resp.inner.inner.0.clicked() {
            state.show_scope = !state.show_scope;
        }

        if resp.inner.inner.1.clicked() {
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

        responses
    }

    fn separator(
        &self,
        ui: &mut egui::Ui,
        _node_id: NodeId,
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
}

impl DataTypeTrait<SynthGraphState> for SynthDataType {
    fn data_type_color(&self, _user_state: &mut SynthGraphState) -> egui::Color32 {
        match self {
            SynthDataType::Float => egui::Color32::LIGHT_BLUE,
            SynthDataType::Midi => egui::Color32::LIGHT_GREEN,
        }
    }

    fn name(&self) -> Cow<str> {
        match self {
            SynthDataType::Float => Cow::Borrowed("signal"),
            SynthDataType::Midi => Cow::Borrowed("MIDI"),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SynthValueType(pub compute::Value);

impl SynthValueType {
    pub fn data_type(&self) -> SynthDataType {
        match &self.0 {
            compute::Value::Float(_) => SynthDataType::Float,
            compute::Value::Midi { .. } => SynthDataType::Midi,
            _ => unimplemented!(),
        }
    }

    pub fn default_with_type(ty: SynthDataType) -> Self {
        SynthValueType(match ty {
            SynthDataType::Float => compute::Value::Float(0.0),
            SynthDataType::Midi => compute::Value::None,
        })
    }
}

impl Eq for SynthValueType {}

impl Default for SynthValueType {
    fn default() -> Self {
        SynthValueType(compute::Value::Float(0.0))
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
        let ui_inputs = user_state.node_ui_inputs.get(&node_id).unwrap();
        if let Some(input) = ui_inputs.get(param_name) {
            ui.horizontal(|ui| {
                ui.label(param_name);
                input.show_always(ui, *node_data.verbose.borrow());
                input.show_disconnected(ui, *node_data.verbose.borrow());
            });
        }

        Default::default()
    }

    fn value_widget_connected(
        &mut self,
        param_name: &str,
        node_id: NodeId,
        ui: &mut egui::Ui,
        user_state: &mut Self::UserState,
        node_data: &Self::NodeData,
    ) -> Vec<Self::Response> {
        let ui_inputs = user_state.node_ui_inputs.get(&node_id).unwrap();
        if let Some(input) = ui_inputs.get(param_name) {
            ui.horizontal(|ui| {
                ui.label(param_name);
                input.show_always(ui, *node_data.verbose.borrow());
            });
        }

        Default::default()
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SynthNodeTemplate {
    template: Box<dyn Node>,
    name: String,
}

impl Clone for SynthNodeTemplate {
    fn clone(&self) -> Self {
        SynthNodeTemplate {
            template: dyn_clone::clone_box(&*self.template),
            name: self.name.clone(),
        }
    }
}

impl NodeTemplateTrait for SynthNodeTemplate {
    type NodeData = SynthNodeData;
    type DataType = SynthDataType;
    type ValueType = SynthValueType;
    type UserState = SynthGraphState;

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
            let out_data_ty = match out.kind {
                compute::ValueKind::Float => SynthDataType::Float,
                compute::ValueKind::Midi => SynthDataType::Midi,
                _ => unimplemented!(),
            };
            graph.add_output_param(node_id, out.name, out_data_ty);
        }

        let mut ui_inputs = HashMap::new();
        for input in node.inputs() {
            let data_type = match input.kind {
                compute::ValueKind::Float => SynthDataType::Float,
                compute::ValueKind::Midi => SynthDataType::Midi,
                _ => unimplemented!(),
            };

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
            all.extend(
                list.all()
                    .into_iter()
                    .map(|(template, name)| SynthNodeTemplate { template, name }),
            )
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
}

impl UserResponseTrait for SynthNodeResponse {}

#[derive(Serialize, Deserialize)]
pub struct SynthCtx {
    pub midi_smf: Vec<SmfSourceNew>,
    pub midi_jack: Vec<JackSourceNew>,
    #[serde(skip)]
    #[serde(default = "Instant::now")]
    last_updated_jack: Instant,
}

impl Default for SynthCtx {
    fn default() -> Self {
        SynthCtx {
            midi_smf: Default::default(),
            midi_jack: Default::default(),
            last_updated_jack: Instant::now(),
        }
    }
}

impl SynthCtx {
    pub fn update_jack(&mut self) {
        if self.last_updated_jack.elapsed().as_secs_f32() < 2.0 {
            return;
        }

        self.last_updated_jack = Instant::now();
        self.midi_jack = JackSourceNew::all();
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
