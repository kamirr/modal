use std::{
    any::Any,
    sync::{mpsc::Sender, Arc, Mutex},
};

use serde::{Deserialize, Serialize};

use crate::graph::{self, SynthCtx};

use super::{Input, Node, NodeConfig, NodeEvent, NodeList};

#[derive(Debug, Default)]
struct MySender(Option<Sender<f32>>);

impl Clone for MySender {
    fn clone(&self) -> Self {
        MySender(None)
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct JackAudioConfig {
    #[serde(skip)]
    sender: Mutex<Option<graph::AudioOut>>,
}

impl NodeConfig for JackAudioConfig {
    fn show(&self, _ui: &mut eframe::egui::Ui, data: &dyn Any) {
        let ctx = data.downcast_ref::<SynthCtx>().unwrap();

        let mut sender = self.sender.lock().unwrap();
        if sender.is_none() {
            *sender = Some(ctx.audio_out.clone());
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct JackAudioOut {
    config: Arc<JackAudioConfig>,
    #[serde(skip)]
    sender: MySender,
}

impl JackAudioOut {
    pub fn new() -> Self {
        JackAudioOut {
            config: Arc::new(JackAudioConfig {
                sender: Mutex::new(None),
            }),
            sender: MySender(None),
        }
    }
}

#[typetag::serde]
impl Node for JackAudioOut {
    fn feed(&mut self, data: &[Option<f32>]) -> Vec<NodeEvent> {
        let sample = data[0].unwrap_or(0.0);

        if let Some(sender) = &self.sender.0 {
            sender.send(sample).ok();
        } else {
            let config_sender = self.config.sender.lock().unwrap();
            if let Some(shared_audio_out) = &*config_sender {
                if let Some(sender) = shared_audio_out.stream.lock().unwrap().take() {
                    self.sender = MySender(Some(sender))
                }
            }
        }

        Default::default()
    }

    fn inputs(&self) -> Vec<Input> {
        vec![Input::new("audio")]
    }

    fn config(&self) -> Option<Arc<dyn NodeConfig>> {
        Some(Arc::clone(&self.config) as Arc<_>)
    }
}

fn jack_audio_out() -> Box<dyn Node> {
    Box::new(JackAudioOut::new())
}

pub struct Output;

impl NodeList for Output {
    fn all(&self) -> Vec<(Box<dyn Node>, String)> {
        vec![(jack_audio_out(), "Jack Audio Output".into())]
    }
}
