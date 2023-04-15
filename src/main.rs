mod compute;
mod graph;
mod remote;

use eframe::egui;
use egui_node_graph::{InputParamKind, NodeId, NodeResponse};

use compute::node::{self, Input, NodeEvent};

fn main() {
    let options = eframe::NativeOptions {
        initial_window_size: Some(egui::vec2(1600.0, 1200.0)),
        ..Default::default()
    };

    eframe::run_native(
        "Modal",
        options,
        Box::new(|_cc| Box::new(SynthApp::default())),
    );
}

struct SynthApp {
    state: graph::SynthEditorState,
    user_state: graph::SynthGraphState,
    all_nodes: graph::AllSynthNodeTemplates,
    remote: remote::RuntimeRemote,
}

impl Default for SynthApp {
    fn default() -> Self {
        pub use node::all::*;

        SynthApp {
            state: Default::default(),
            user_state: Default::default(),
            all_nodes: graph::AllSynthNodeTemplates::new(vec![Box::new(Basic), Box::new(Noise)]),
            remote: Default::default(),
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
                self.state.graph.add_input_param(
                    node_id,
                    input.name.clone(),
                    graph::SynthDataType,
                    graph::SynthValueType(0.0),
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
                NodeResponse::User(graph::SynthNodeResponse::SetActiveNode(id)) => {
                    println!("set active {id:?}");
                    self.user_state.active_node = Some(id);
                    self.remote.record(Some(id));
                }
                NodeResponse::User(graph::SynthNodeResponse::ClearActiveNode) => {
                    println!("unset active");
                    self.user_state.active_node = None;
                    self.remote.record(None);
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

        self.remote.wait();
        ctx.request_repaint();
    }
}
