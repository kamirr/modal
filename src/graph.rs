use std::{
    borrow::Cow,
    collections::HashMap,
    sync::{Arc, Weak},
};

use egui_node_graph::{
    DataTypeTrait, Graph, GraphEditorState, InputParamKind, NodeDataTrait, NodeId, NodeResponse,
    NodeTemplateIter, NodeTemplateTrait, UserResponseTrait, WidgetValueTrait,
};

use eframe::egui;

use crate::compute::node::{InputUi, Node, NodeConfig, NodeList};

#[derive(Debug)]
pub struct SynthNodeData;

impl NodeDataTrait for SynthNodeData {
    type Response = SynthNodeResponse;
    type UserState = SynthGraphState;
    type DataType = SynthDataType;
    type ValueType = SynthValueType;

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
            config.show(ui);
        }

        if !is_active {
            if ui.button("👁 Set active").clicked() {
                responses.push(NodeResponse::User(SynthNodeResponse::SetActiveNode(
                    node_id,
                )));
            }
        } else {
            let button =
                egui::Button::new(egui::RichText::new("👁 Active").color(egui::Color32::BLACK))
                    .fill(egui::Color32::GOLD);
            if ui.add(button).clicked() {
                responses.push(NodeResponse::User(SynthNodeResponse::ClearActiveNode));
            }
        }

        responses
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SynthDataType;

impl DataTypeTrait<SynthGraphState> for SynthDataType {
    fn data_type_color(&self, _user_state: &mut SynthGraphState) -> egui::Color32 {
        egui::Color32::LIGHT_BLUE
    }

    fn name(&self) -> Cow<str> {
        Cow::Borrowed("signal")
    }
}

#[derive(Clone, Debug, PartialEq)]
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
        _node_data: &Self::NodeData,
    ) -> Vec<Self::Response> {
        let ui_inputs = user_state.node_ui_inputs.get(&node_id).unwrap();
        ui.horizontal(|ui| {
            ui.label(param_name);
            if let Some(input) = ui_inputs.get(param_name) {
                input.show(ui);
            }
        });

        Default::default()
    }
}

#[derive(Clone, Copy, Debug)]
pub struct SynthNodeTemplate {
    new: fn() -> Box<dyn Node>,
    name: &'static str,
}

impl NodeTemplateTrait for SynthNodeTemplate {
    type NodeData = SynthNodeData;
    type DataType = SynthDataType;
    type ValueType = SynthValueType;
    type UserState = SynthGraphState;

    fn node_finder_label(&self, _user_state: &mut Self::UserState) -> Cow<str> {
        Cow::Borrowed(self.name)
    }

    fn node_graph_label(&self, user_state: &mut Self::UserState) -> String {
        self.node_finder_label(user_state).into()
    }

    fn user_data(&self, _user_state: &mut Self::UserState) -> Self::NodeData {
        SynthNodeData
    }

    fn build_node(
        &self,
        graph: &mut Graph<Self::NodeData, Self::DataType, Self::ValueType>,
        user_state: &mut Self::UserState,
        node_id: NodeId,
    ) {
        let node: Box<dyn Node> = (self.new)();

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
                    .map(|(new, name)| SynthNodeTemplate { new, name }),
            )
        }

        all
    }
}

#[derive(Clone, Debug)]
pub enum SynthNodeResponse {
    SetActiveNode(NodeId),
    ClearActiveNode,
}

impl UserResponseTrait for SynthNodeResponse {}

#[derive(Default)]
pub struct SynthGraphState {
    pub active_node: Option<NodeId>,
    pub node_ui_inputs: HashMap<NodeId, HashMap<String, Arc<dyn InputUi>>>,
    pub node_configs: HashMap<NodeId, Weak<dyn NodeConfig>>,
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
