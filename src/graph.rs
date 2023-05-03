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

use eframe::egui;
use serde::{Deserialize, Serialize};

use crate::{
    compute::node::{
        all::source::{jack::JackSourceNew, smf::SmfSourceNew},
        InputUi, Node, NodeConfig, NodeList,
    },
    scope::Scope,
    util::toggle_button,
};

#[derive(Debug, Serialize, Deserialize)]
pub struct SynthNodeData {
    pub scope: RefCell<Option<Scope>>,
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

        let mut responses = vec![];
        let is_active = user_state
            .active_node
            .map(|id| id == node_id)
            .unwrap_or(false);

        if let Some(config) = user_state
            .node_configs
            .get(&node_id)
            .and_then(|wk| wk.upgrade())
        {
            config.show(ui, &mut user_state.ctx);
        }

        let show_scope = ui
            .horizontal(|ui| {
                if !is_active {
                    if ui.button("👂Play").clicked() {
                        responses.push(NodeResponse::User(SynthNodeResponse::SetActiveNode(
                            node_id,
                        )));
                    }
                } else {
                    let button = egui::Button::new(
                        egui::RichText::new("👂Play").color(egui::Color32::BLACK),
                    )
                    .fill(egui::Color32::GOLD);
                    if ui.add(button).clicked() {
                        responses.push(NodeResponse::User(SynthNodeResponse::ClearActiveNode));
                    }
                }

                if self.scope.borrow().is_none() {
                    if ui.button("👁Scope").clicked() {
                        *self.scope.borrow_mut() = Some(Scope::new());
                        responses.push(NodeResponse::User(SynthNodeResponse::StartRecording(
                            node_id,
                        )));

                        true
                    } else {
                        false
                    }
                } else {
                    let button = egui::Button::new(
                        egui::RichText::new("👁Scope").color(egui::Color32::BLACK),
                    )
                    .fill(egui::Color32::GOLD);
                    if ui.add(button).clicked() {
                        *self.scope.borrow_mut() = None;
                        responses.push(NodeResponse::User(SynthNodeResponse::StopRecording(
                            node_id,
                        )));

                        false
                    } else {
                        true
                    }
                }
            })
            .inner;

        if show_scope {
            self.scope.borrow_mut().as_mut().unwrap().show(ui);
        }

        responses
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SynthDataType;

impl DataTypeTrait<SynthGraphState> for SynthDataType {
    fn data_type_color(&self, _user_state: &mut SynthGraphState) -> egui::Color32 {
        egui::Color32::LIGHT_BLUE
    }

    fn name(&self) -> Cow<str> {
        Cow::Borrowed("signal")
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SynthValueType(pub f32);

impl Eq for SynthValueType {}

impl Default for SynthValueType {
    fn default() -> Self {
        SynthValueType(0.0)
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
            input.show_disconnected(ui, *node_data.verbose.borrow());
        }

        Default::default()
    }

    fn value_widget_always(
        &mut self,
        param_name: &str,
        node_id: NodeId,
        ui: &mut egui::Ui,
        user_state: &mut Self::UserState,
        node_data: &Self::NodeData,
    ) -> Vec<Self::Response> {
        ui.label(param_name);

        let ui_inputs = user_state.node_ui_inputs.get(&node_id).unwrap();
        if let Some(input) = ui_inputs.get(param_name) {
            ui.push_id(param_name, |ui| {
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
            scope: RefCell::new(None),
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

        let input_signal = |graph: &mut SynthGraph, name: String| {
            graph.add_input_param(
                node_id,
                name,
                SynthDataType,
                SynthValueType(0.0),
                InputParamKind::ConnectionOrConstant,
                true,
            );
        };

        graph.add_output_param(node_id, "".to_string(), SynthDataType);

        let mut ui_inputs = HashMap::new();
        for input in node.inputs() {
            input_signal(graph, input.name.clone());
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
    SetActiveNode(NodeId),
    ClearActiveNode,
    StartRecording(NodeId),
    StopRecording(NodeId),
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
    pub active_node: Option<NodeId>,
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
