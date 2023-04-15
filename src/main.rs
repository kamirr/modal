mod compute;

use std::{
    borrow::Cow,
    collections::HashMap,
    sync::{
        mpsc::{channel, Receiver, Sender, TryRecvError},
        Arc, Weak,
    },
};

use bimap::BiHashMap;
use compute::{
    node::{all::*, Input, InputUi, Node, NodeConfig, NodeEvent, NodeList},
    Runtime,
};
use egui_node_graph::{
    DataTypeTrait, Graph, GraphEditorState, InputParamKind, NodeDataTrait, NodeId, NodeResponse,
    NodeTemplateIter, NodeTemplateTrait, UserResponseTrait, WidgetValueTrait,
};
use thunderdome::Index;

use eframe::egui;

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
pub struct SynthValueType(f32);

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

impl Default for AllSynthNodeTemplates {
    fn default() -> Self {
        AllSynthNodeTemplates {
            lists: vec![Box::new(Basic)],
        }
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
    active_node: Option<NodeId>,
    node_ui_inputs: HashMap<NodeId, HashMap<String, Arc<dyn InputUi>>>,
    node_configs: HashMap<NodeId, Weak<dyn NodeConfig>>,
    nodes: HashMap<NodeId, Box<dyn Node>>,
}

type SynthGraph = Graph<SynthNodeData, SynthDataType, SynthValueType>;
type SynthEditorState = GraphEditorState<
    SynthNodeData,
    SynthDataType,
    SynthValueType,
    SynthNodeTemplate,
    SynthGraphState,
>;

#[derive(Debug)]
pub enum RtRequest {
    Insert {
        id: NodeId,
        inputs: Vec<Option<Index>>,
        node: Box<dyn Node>,
    },
    Remove(Index),
    SetInput {
        src: Option<Index>,
        dst: Index,
        port: usize,
    },
    SetAllInputs {
        dst: Index,
        inputs: Vec<Option<Index>>,
    },
    Play(Option<Index>),
}

pub enum RtResponse {
    Inserted(NodeId, Index),
    NodeEvents(Vec<(Index, Vec<NodeEvent>)>),
    Step,
}

struct RuntimeRemote {
    tx: Sender<RtRequest>,
    rx: Receiver<RtResponse>,
    must_wait: bool,
    mapping: BiHashMap<NodeId, Index>,
    node_events: Vec<(Index, Vec<NodeEvent>)>,
}

impl RuntimeRemote {
    pub fn start() -> Self {
        let (cmd_tx, cmd_rx) = channel();
        let (resp_tx, resp_rx) = channel();

        let mut rt = Runtime::new();

        let mut record = None;
        let buf_size = 512;
        let mut buf = vec![0.0; buf_size];

        let (stream, handle) = rodio::OutputStream::try_default().unwrap();
        std::mem::forget(stream);

        let sink = rodio::Sink::try_new(&handle).unwrap();
        while sink.len() as f32 * buf_size as f32 / 44100.0 < 0.1 {
            let source = rodio::buffer::SamplesBuffer::new(1, 44100, buf.clone());
            sink.append(source);
        }
        sink.play();

        std::thread::spawn(move || loop {
            while sink.len() as f32 * buf_size as f32 / 44100.0 < 0.1 {
                if let Some(record) = record {
                    for k in 0..buf_size {
                        let evs = rt.step();
                        if evs.len() > 0 {
                            resp_tx.send(RtResponse::NodeEvents(evs)).ok();
                        }

                        buf[k] = rt.peek(record);
                    }
                } else {
                    for k in 0..buf_size {
                        let evs = rt.step();
                        if evs.len() > 0 {
                            resp_tx.send(RtResponse::NodeEvents(evs)).ok();
                        }

                        buf[k] = 0.0;
                    }
                }

                let source = rodio::buffer::SamplesBuffer::new(1, 44100, buf.clone());
                sink.append(source);
            }

            let cmd = match cmd_rx.try_recv() {
                Ok(cmd) => cmd,
                Err(TryRecvError::Empty) => continue,
                Err(TryRecvError::Disconnected) => return,
            };

            match cmd {
                RtRequest::Insert { id, inputs, node } => {
                    let idx = rt.insert(inputs, node);
                    resp_tx.send(RtResponse::Inserted(id, idx)).ok();
                }
                RtRequest::Play(node) => {
                    record = node;
                    eprintln!("playback from {:?}", record);
                }
                RtRequest::SetInput { src, dst, port } => {
                    rt.set_input(dst, port, src);
                }
                RtRequest::SetAllInputs { dst, inputs } => {
                    rt.set_all_inputs(dst, inputs);
                }
                RtRequest::Remove(index) => {
                    rt.remove(index);
                }
            }

            resp_tx.send(RtResponse::Step).ok();
        });

        RuntimeRemote {
            tx: cmd_tx,
            rx: resp_rx,
            must_wait: false,
            mapping: BiHashMap::new(),
            node_events: Vec::new(),
        }
    }

    pub fn insert(&mut self, id: NodeId, node: Box<dyn Node>) {
        let inputs = vec![None; node.inputs().len()];
        self.tx.send(RtRequest::Insert { id, inputs, node }).ok();
        self.must_wait = true;
    }

    pub fn remove(&mut self, id: NodeId) {
        let idx = self.mapping.get_by_left(&id).cloned().unwrap();
        self.tx.send(RtRequest::Remove(idx)).ok();
        self.mapping.remove_by_left(&id);
        self.must_wait = true;
    }

    pub fn set_inputs(&mut self, dst: NodeId, inputs: Vec<Option<Index>>) {
        self.tx
            .send(RtRequest::SetAllInputs {
                dst: *self.mapping.get_by_left(&dst).unwrap(),
                inputs,
            })
            .ok();
    }

    pub fn connect(&mut self, src: NodeId, dst: NodeId, port: usize) {
        let src = self.mapping.get_by_left(&src).cloned().unwrap();
        let dst = self.mapping.get_by_left(&dst).cloned().unwrap();
        self.tx
            .send(RtRequest::SetInput {
                src: Some(src),
                dst,
                port,
            })
            .ok();
    }

    pub fn disconnect(&mut self, dst: NodeId, port: usize) {
        let dst = self.mapping.get_by_left(&dst).cloned().unwrap();
        self.tx
            .send(RtRequest::SetInput {
                src: None,
                dst,
                port,
            })
            .ok();
    }

    pub fn record(&mut self, id: Option<NodeId>) {
        let idx = id.and_then(|id| self.mapping.get_by_left(&id).cloned());
        self.tx.send(RtRequest::Play(idx)).ok();
        // doesn't need to set must_wait
    }

    pub fn process(&mut self, resp: RtResponse) {
        match resp {
            RtResponse::Inserted(id, idx) => {
                self.mapping.insert(id, idx);
            }
            RtResponse::NodeEvents(evs) => {
                self.node_events.extend(evs.into_iter());
            }
            RtResponse::Step => {}
        }
    }

    pub fn events(&mut self) -> Vec<(Index, Vec<NodeEvent>)> {
        std::mem::take(&mut self.node_events)
    }

    pub fn wait(&mut self) {
        if self.must_wait {
            while let Ok(resp) = self.rx.recv() {
                if let RtResponse::Step = &resp {
                    break;
                }

                self.process(resp);
            }

            self.must_wait = false;
        }

        while let Ok(resp) = self.rx.try_recv() {
            self.process(resp);
        }
    }
}

impl Default for RuntimeRemote {
    fn default() -> Self {
        RuntimeRemote::start()
    }
}

fn main() {
    let options = eframe::NativeOptions {
        initial_window_size: Some(egui::vec2(1600.0, 1200.0)),
        ..Default::default()
    };

    eframe::run_native(
        "My egui App",
        options,
        Box::new(|_cc| Box::new(SynthApp::default())),
    );
}

#[derive(Default)]
struct SynthApp {
    state: SynthEditorState,
    user_state: SynthGraphState,
    all_nodes: AllSynthNodeTemplates,
    remote: RuntimeRemote,
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
                self.state.graph.add_input_param(
                    node_id,
                    input.name.clone(),
                    SynthDataType,
                    SynthValueType(0.0),
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
                .and_then(|node_id| self.remote.mapping.get_by_left(&node_id))
                .cloned();
            rt_inputs.push(src);
        }
        self.remote.set_inputs(node_id, rt_inputs);
    }
}

impl eframe::App for SynthApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::TopBottomPanel::top("top").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                egui::widgets::global_dark_light_mode_switch(ui);
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
                NodeResponse::User(SynthNodeResponse::SetActiveNode(id)) => {
                    println!("set active {id:?}");
                    self.user_state.active_node = Some(id);
                    self.remote.record(Some(id));
                }
                NodeResponse::User(SynthNodeResponse::ClearActiveNode) => {
                    println!("unset active");
                    self.user_state.active_node = None;
                    self.remote.record(None);
                }
                _ => {}
            }
        }

        for (idx, evs) in self.remote.events() {
            let Some(&node_id) = self.remote.mapping.get_by_right(&idx) else {
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

        self.remote.wait();
        ctx.request_repaint();
    }
}
