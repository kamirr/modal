mod compute;

use std::{
    borrow::Cow,
    collections::HashMap,
    sync::mpsc::{channel, Receiver, Sender, TryRecvError},
};

use bimap::BiHashMap;
use compute::{
    node::{all::*, Node, NodeList, Param, ParamSignature, ParamValue, WithParam},
    Runtime,
};
use egui_node_graph::{
    DataTypeTrait, Graph, GraphEditorState, InputParamKind, NodeDataTrait, NodeId, NodeResponse,
    NodeTemplateIter, NodeTemplateTrait, UserResponseTrait, WidgetValueTrait,
};
use thunderdome::Index;
use wav::WAV_FORMAT_IEEE_FLOAT;

use eframe::egui::{self, DragValue};

fn _old_main() {
    let mut rt = Runtime::new();

    let net = feedback_many(
        (
            chain2(
                delay().with_param(44100 / 440),
                fir().with_param((0.5, 0.5)),
            ),
            chain2(
                delay().with_param(44100 / 660),
                fir().with_param((0.5, 0.5)),
            ),
        ),
        (
            dot().with_param((0.99, 0.01)),
            dot().with_param((0.01, 0.99)),
        ),
        add().with_param(2),
    );

    let in1 = rt.insert([], constant());
    let in2 = rt.insert([], constant());
    let net_out = rt.insert([in1, in2], chain2(net, gain().with_param(1.5)));

    let data: Vec<_> = (0..44100 * 2)
        .map(|_| {
            rt.step();

            rt.peek(net_out)
        })
        .collect();

    let header = wav::Header::new(WAV_FORMAT_IEEE_FLOAT, 1, 44100, 32);
    let mut out = std::fs::File::create("out.wav").unwrap();

    wav::write(header, &wav::BitDepth::ThirtyTwoFloat(data), &mut out).unwrap();
}

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
pub enum SynthDataType {
    Signal,
    Param(ParamSignature),
}

impl DataTypeTrait<SynthGraphState> for SynthDataType {
    fn data_type_color(&self, _user_state: &mut SynthGraphState) -> egui::Color32 {
        match self {
            SynthDataType::Signal => egui::Color32::LIGHT_BLUE,
            SynthDataType::Param(_) => egui::Color32::RED,
        }
    }

    fn name(&self) -> Cow<str> {
        Cow::Borrowed(match self {
            SynthDataType::Signal => "signal",
            SynthDataType::Param(_) => "param",
        })
    }
}

pub enum SynthValueType {
    Signal(f32),
    Param(Param),
}

impl Default for SynthValueType {
    fn default() -> Self {
        SynthValueType::Signal(0.0)
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
        let drag_f32_clamped = |ui: &mut egui::Ui, value: &mut f32| {
            ui.add(
                DragValue::new(value)
                    .speed(0.01)
                    .fixed_decimals(2)
                    .clamp_range(-1.0..=1.0),
            );
        };

        let drag_f32 = |ui: &mut egui::Ui, value: &mut f32| {
            ui.add(DragValue::new(value).speed(0.01).fixed_decimals(2));
        };

        let drag_u32 = |ui: &mut egui::Ui, value: &mut u32| {
            ui.add(DragValue::new(value).clamp_range(0..=u32::MAX))
        };

        let show_1 = |ui: &mut egui::Ui, pv: &mut ParamValue, name: &str| {
            match pv {
                ParamValue::F(value) => {
                    ui.horizontal(|ui| {
                        ui.label(name);
                        drag_f32(ui, value)
                    });
                }
                ParamValue::FDyn(values) => {
                    egui::CollapsingHeader::new(name).show(ui, |ui| {
                        for (i, value) in values.iter_mut().enumerate() {
                            ui.horizontal(|ui| {
                                ui.label(format!("#{i}"));
                                drag_f32(ui, value);
                            });
                        }

                        ui.horizontal(|ui| {
                            if ui.button("add").clicked() {
                                values.push(0.0);
                            }
                            if ui.button("pop").clicked() {
                                values.pop();
                            }
                        })
                    });
                }
                ParamValue::U(value) => {
                    ui.horizontal(|ui| {
                        ui.label(name);
                        drag_u32(ui, value);
                    });
                }
            };
        };

        match self {
            SynthValueType::Signal(value) => {
                ui.horizontal(|ui| {
                    ui.label(param_name);
                    drag_f32_clamped(ui, value);
                });
            }
            SynthValueType::Param(param) => {
                if let Some(params) = user_state.param_states.get(&node_id) {
                    let sig = params
                        .iter()
                        .find(|(name, _sig, _param)| name == param_name)
                        .map(|(_name, sig, _param)| sig)
                        .unwrap();

                    match param.0.as_mut_slice() {
                        [] => {}
                        [pv] => show_1(ui, pv, param_name),
                        pvs => {
                            egui::CollapsingHeader::new(param_name).show(ui, |ui| {
                                for (i, pv) in pvs.iter_mut().enumerate() {
                                    show_1(ui, pv, &sig.0[i].name.as_ref());
                                }
                            });
                        }
                    }
                }
            }
        };

        vec![]
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
                SynthDataType::Signal,
                SynthValueType::Signal(0.0),
                InputParamKind::ConnectionOrConstant,
                true,
            );
        };

        let input_param =
            |graph: &mut SynthGraph, name: String, sig: ParamSignature, value: SynthValueType| {
                let data_type = SynthDataType::Param(sig);
                graph.add_input_param(
                    node_id,
                    name,
                    data_type,
                    value,
                    InputParamKind::ConstantOnly,
                    true,
                );
            };

        graph.add_output_param(node_id, "".to_string(), SynthDataType::Signal);

        let meta = node.meta();
        let param_values = node.get_param();

        user_state.nodes.insert(node_id, node);

        for input in meta.inputs {
            input_signal(graph, input);
        }

        for ((name, layout), param_value) in meta.params.into_iter().zip(param_values.into_iter()) {
            input_param(graph, name, layout, SynthValueType::Param(param_value));
        }
    }
}

pub struct AllSynthNodeTemplates {
    lists: Vec<Box<dyn NodeList>>,
}

impl Default for AllSynthNodeTemplates {
    fn default() -> Self {
        AllSynthNodeTemplates {
            lists: vec![Box::new(Basic), Box::new(Filters)],
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
    param_states: HashMap<NodeId, Vec<(String, ParamSignature, Param)>>,
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

pub enum RtRequest {
    SetParam {
        index: Index,
        param: Vec<Param>,
    },
    Insert {
        id: NodeId,
        inputs: Vec<Index>,
        node: Box<dyn Node>,
    },
    Remove(Index),
    SetInput {
        src: Index,
        dst: Index,
        port: usize,
    },
    Play(Option<Index>),
}

pub enum RtResponse {
    ParamUpd(Index, Vec<(String, ParamSignature, Param)>),
    Inserted(NodeId, Index),
    Step,
}

struct RuntimeRemote {
    tx: Sender<RtRequest>,
    rx: Receiver<RtResponse>,
    default: Index,
    must_wait: bool,
    mapping: BiHashMap<NodeId, Index>,
    param_updates: Vec<(NodeId, Vec<(String, ParamSignature, Param)>)>,
}

impl RuntimeRemote {
    pub fn start() -> Self {
        let (cmd_tx, cmd_rx) = channel();
        let (resp_tx, resp_rx) = channel();

        let mut rt = Runtime::new();
        let default = rt.insert([], constant());

        let mut record = None;
        let buf_size = 512;
        let mut buf = vec![0.0; buf_size];
        std::thread::spawn(move || loop {
            let cmd = match cmd_rx.try_recv() {
                Ok(cmd) => cmd,
                Err(TryRecvError::Empty) => continue,
                Err(TryRecvError::Disconnected) => return,
            };

            match cmd {
                RtRequest::Insert { id, inputs, node } => {
                    let idx = rt.insert_box(inputs, node);
                    resp_tx.send(RtResponse::Inserted(id, idx)).ok();
                    resp_tx
                        .send(RtResponse::ParamUpd(idx, rt.get_param(idx)))
                        .ok();
                }
                RtRequest::Play(node) => {
                    record = node;
                    eprintln!("playback from {:?}", record);
                }
                RtRequest::SetInput { src, dst, port } => {
                    rt.set_input(dst, port, src);
                }
                RtRequest::SetParam { index, param } => {
                    rt.set_param(index, param);
                    let got_param = rt.get_param(index);
                    resp_tx.send(RtResponse::ParamUpd(index, got_param)).ok();
                }
                RtRequest::Remove(index) => {
                    rt.remove(index);
                }
            }

            for k in 0..buf_size {
                rt.step();
                if let Some(record) = record {
                    buf[k] = rt.peek(record);
                }
            }

            resp_tx.send(RtResponse::Step).ok();
        });

        RuntimeRemote {
            tx: cmd_tx,
            rx: resp_rx,
            default,
            must_wait: false,
            mapping: BiHashMap::new(),
            param_updates: Vec::new(),
        }
    }

    pub fn insert(&mut self, id: NodeId, node: Box<dyn Node>) {
        let inputs = vec![self.default; node.meta().inputs.len()];
        self.tx.send(RtRequest::Insert { id, inputs, node }).ok();
        self.must_wait = true;
    }

    pub fn remove(&mut self, id: NodeId) {
        let idx = self.mapping.get_by_left(&id).cloned().unwrap();
        self.tx.send(RtRequest::Remove(idx)).ok();
        self.mapping.remove_by_left(&id);
        self.param_updates.retain(|pu| pu.0 != id);
        self.must_wait = true;
    }

    pub fn connect(&mut self, src: NodeId, dst: NodeId, port: usize) {
        let src = self.mapping.get_by_left(&src).cloned().unwrap();
        let dst = self.mapping.get_by_left(&dst).cloned().unwrap();
        self.tx.send(RtRequest::SetInput { src, dst, port }).ok();
    }

    pub fn disconnect(&mut self, dst: NodeId, port: usize) {
        let dst = self.mapping.get_by_left(&dst).cloned().unwrap();
        self.tx
            .send(RtRequest::SetInput {
                src: self.default,
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
            RtResponse::ParamUpd(idx, param) => {
                let id = *self.mapping.get_by_right(&idx).unwrap();
                self.param_updates.push((id, param));
            }
            RtResponse::Step => {}
        }
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
                    let node = self.user_state.nodes.remove(&id).unwrap();
                    self.remote.insert(id, node);
                }
                NodeResponse::DeleteNodeFull { node_id, .. } => {
                    self.remote.remove(node_id);
                }
                NodeResponse::DisconnectEvent { input, .. } => {
                    let in_param = self.state.graph.get_input(input);
                    let in_node_id = in_param.node;
                    let in_node = self.state.graph.nodes.get(in_node_id).unwrap();
                    let in_idx = in_node
                        .input_ids()
                        .enumerate()
                        .find(|(_i, id)| id == &in_param.id)
                        .unwrap()
                        .0;
                    self.remote.disconnect(in_node_id, in_idx);
                    println!("disconnect from {in_node_id:?}:{in_idx:?}")
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
                    self.remote.connect(out_node_id, in_node_id, in_idx);
                    println!("connect {out_node_id:?} to {in_node_id:?}:{in_idx:?}")
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

        for (node_id, param_states) in self.remote.param_updates.drain(..) {
            self.user_state.param_states.insert(node_id, param_states);
        }

        self.remote.wait();

        if let Some(_node) = self.user_state.active_node {}
    }
}
