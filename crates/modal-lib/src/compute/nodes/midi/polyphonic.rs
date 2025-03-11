use std::{
    fmt::Debug,
    sync::{
        atomic::{AtomicBool, AtomicU8, Ordering},
        Arc, Mutex,
    },
};

use eframe::egui::DragValue;
use runtime::{
    node::{Input, Node, NodeConfig, NodeEvent},
    ExternInputs, OutputPort, Runtime, Value, ValueKind,
};
use serde::{Deserialize, Serialize};
use thunderdome::Index;

use crate::{
    compute::inputs::midi::MidiInput,
    editor::{GraphEditor, GraphEditorState, ManagedEditor, SharedEditorData},
    graph::SynthCtx,
    remote::{
        stream_audio_out::{StreamAudioOut, StreamReader},
        ExternInput, RtRequest, RuntimeRemote,
    },
};

#[allow(clippy::large_enum_variant)]
enum LazyEditor {
    Pending(Option<GraphEditorState>),
    Ready {
        editor: Arc<SharedEditorData>,
        reader: Option<StreamReader>,
    },
}

impl LazyEditor {
    fn initialize(&mut self) {
        match self {
            LazyEditor::Pending(state) => {
                let (audio_out, reader) = StreamAudioOut::new();
                let remote = RuntimeRemote::start(Box::new(audio_out));
                remote
                    .tx
                    .send(RtRequest::ExternDefine {
                        input: ExternInput::Midi,
                        kind: ValueKind::Midi,
                    })
                    .ok();
                let mut editor = GraphEditor::new(remote);
                if let Some(state) = state {
                    editor.replace(state.clone());
                }
                *self = LazyEditor::Ready {
                    editor: Arc::new(SharedEditorData::new(editor)),
                    reader: Some(reader),
                };
            }
            LazyEditor::Ready { .. } => {}
        }
    }

    fn serializable_state(&mut self) -> Box<dyn erased_serde::Serialize + '_> {
        match self {
            LazyEditor::Pending(state) => Box::new(state.clone()),
            LazyEditor::Ready { editor, .. } => {
                Box::new(editor.editor.lock().unwrap().serializable_state())
            }
        }
    }

    fn editor(&mut self) -> &Arc<SharedEditorData> {
        self.initialize();

        match self {
            LazyEditor::Ready { editor, .. } => editor,
            LazyEditor::Pending(_) => unreachable!(),
        }
    }

    fn reader(&mut self) -> &mut Option<StreamReader> {
        self.initialize();

        match self {
            LazyEditor::Ready { reader, .. } => reader,
            LazyEditor::Pending(_) => unreachable!(),
        }
    }
}

struct PolyphonicInstrumentConf {
    editor: Mutex<LazyEditor>,
    editor_notified: AtomicBool,
    voices: AtomicU8,
    topology_changed: Mutex<Option<Arc<AtomicBool>>>,
}

#[derive(Serialize)]
struct PolyphonicInstrumentConfSerialize<EDITOR>
where
    EDITOR: Serialize,
{
    editor: EDITOR,
    voices: u8,
}

#[derive(Deserialize)]
struct PolyphonicInstrumentConfDeserialize {
    editor: GraphEditorState,
    voices: u8,
}

impl Debug for PolyphonicInstrumentConf {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PolyphonicInstrumentConf")
            .field("editor", &"hidden")
            .field("voices", &self.voices)
            .finish()
    }
}

impl Serialize for PolyphonicInstrumentConf {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut guard = self.editor.lock().unwrap();
        let state = PolyphonicInstrumentConfSerialize {
            editor: guard.serializable_state(),
            voices: self.voices.load(Ordering::Relaxed),
        };

        state.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for PolyphonicInstrumentConf {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        PolyphonicInstrumentConfDeserialize::deserialize(deserializer).map(|state| {
            PolyphonicInstrumentConf {
                editor: Mutex::new(LazyEditor::Pending(Some(state.editor))),
                editor_notified: AtomicBool::new(false),
                voices: AtomicU8::new(state.voices),
                topology_changed: Mutex::new(None),
            }
        })
    }
}

impl NodeConfig for PolyphonicInstrumentConf {
    fn show(&self, ui: &mut eframe::egui::Ui, data: &dyn std::any::Any) {
        let mut voices = self.voices.load(Ordering::Relaxed);

        let mut topology_changed_guard = self.topology_changed.lock().unwrap();
        if topology_changed_guard.is_none() {
            *topology_changed_guard = Some(Arc::clone(
                &self.editor.lock().unwrap().editor().topology_changed,
            ));
        }

        ui.horizontal(|ui| {
            ui.label("voices");

            let response = ui.add(DragValue::new(&mut voices).range(0..=u8::MAX));

            if response.changed() {
                self.voices.store(voices, Ordering::Relaxed);
            }
        });

        if ui.button("Edit Assembly").clicked() {
            println!("set active editor");
            let editor = Arc::clone(self.editor.lock().unwrap().editor());
            *data
                .downcast_ref::<SynthCtx>()
                .unwrap()
                .visit_editor
                .lock()
                .unwrap() = Some(editor);
        }

        if !self.editor_notified.swap(true, Ordering::SeqCst) {
            println!("append editor");
            let editor = Arc::clone(self.editor.lock().unwrap().editor());
            data.downcast_ref::<SynthCtx>()
                .unwrap()
                .new_editors
                .lock()
                .unwrap()
                .push(ManagedEditor::new("Subassembly", editor));
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct Voice {
    ord: u32,
    key: u8,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct MidiSplitter {
    voices: Vec<Option<Voice>>,
}

impl MidiSplitter {
    fn new(size: usize) -> Self {
        MidiSplitter {
            voices: vec![None; size],
        }
    }

    fn route(&mut self, msg: midly::MidiMessage, mut sink: impl FnMut(usize, midly::MidiMessage)) {
        match msg {
            midly::MidiMessage::NoteOn { key, .. } => {
                // Bump ord of all active voices
                self.voices
                    .iter_mut()
                    .filter_map(|v| v.as_mut())
                    .for_each(|v| v.ord += 1);

                // Select the voice with highest ord (oldest one) or an inactive
                // one if available
                if let Some((n, voice)) = self
                    .voices
                    .iter_mut()
                    .enumerate()
                    .max_by_key(|(_, voice)| voice.as_ref().map(|v| v.ord).unwrap_or(u32::MAX))
                {
                    sink(n, msg);
                    *voice = Some(Voice {
                        ord: 0,
                        key: key.as_int(),
                    });
                }
            }
            midly::MidiMessage::NoteOff { key, .. } => {
                for (n, voice_slot) in self.voices.iter_mut().enumerate() {
                    if let Some(voice) = voice_slot {
                        if voice.key == key.as_int() {
                            sink(n, msg);
                            *voice_slot = None;
                            break;
                        }
                    }
                }
            }
            midly::MidiMessage::Aftertouch { key, .. } => {
                for (n, voice_slot) in self.voices.iter_mut().enumerate() {
                    if let Some(voice) = voice_slot {
                        if voice.key == key.as_int() {
                            sink(n, msg);
                            break;
                        }
                    }
                }
            }
            midly::MidiMessage::Controller { .. }
            | midly::MidiMessage::ProgramChange { .. }
            | midly::MidiMessage::ChannelAftertouch { .. }
            | midly::MidiMessage::PitchBend { .. } => {
                for n in 0..self.voices.len() {
                    sink(n, msg)
                }
            }
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct VoiceRuntime {
    runtime: Runtime,
    ext_in_handle: u64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PolyphonicInstrument {
    conf: Arc<PolyphonicInstrumentConf>,
    #[serde(skip)]
    template_reader: Option<StreamReader>,
    template_reader_queue: usize,
    midi_in: Arc<MidiInput>,
    runtimes: Vec<VoiceRuntime>,
    splitter: MidiSplitter,
    #[serde(skip)]
    playback_port: Option<OutputPort>,
    out: f32,
}

// TODO: Clone impl should do a deep copy of PolyphonicInstrumentConf. As it stands,
// if PolyphonicInstrument were cloned and feed were to be called on both instances,
// one would panic.
impl Clone for PolyphonicInstrument {
    fn clone(&self) -> Self {
        PolyphonicInstrument {
            conf: self.conf.clone(),
            template_reader: None,
            template_reader_queue: 0,
            midi_in: self.midi_in.clone(),
            runtimes: self.runtimes.clone(),
            splitter: self.splitter.clone(),
            playback_port: self.playback_port,
            out: self.out,
        }
    }
}

#[typetag::serde]
impl Node for PolyphonicInstrument {
    fn feed(&mut self, _inputs: &ExternInputs, data: &[Value]) -> Vec<NodeEvent> {
        // The edited template must be synchronized properly for the editor to work.
        // The output is discarded
        if self.template_reader.is_none() {
            self.template_reader = self.conf.editor.lock().unwrap().reader().take();
        }
        let Some(reader) = &self.template_reader else {
            panic!()
        };
        if self.template_reader_queue == 0 {
            self.template_reader_queue = reader.read().len();
        }
        self.template_reader_queue -= 1;

        // Do the actual work
        let voice_count = self.conf.voices.load(Ordering::Relaxed) as usize;
        let topology_changed = || {
            self.conf
                .topology_changed
                .lock()
                .unwrap()
                .as_ref()
                .map(|arc| arc.swap(false, Ordering::Relaxed))
                .unwrap_or_default()
        };
        if self.runtimes.len() != voice_count || topology_changed() {
            println!("rebuild voices - begin");
            let (play_node, (base_rt, mapping)) = {
                let mut lazy_editor_guard = self.conf.editor.lock().unwrap();
                let mut editor_guard = lazy_editor_guard.editor().editor.lock().unwrap();
                let play_node = editor_guard.user_state.rt_playback;
                let (base_rt, mapping) = editor_guard.get_runtime();

                (play_node, (base_rt, mapping))
            };
            println!("rebuild voices - end");
            self.playback_port = play_node
                .map(|(play_id, port)| {
                    (
                        mapping
                            .iter()
                            .copied()
                            .find_map(|(node_id, idx)| (node_id == play_id).then_some(idx))
                            .map(Index::from_bits)
                            .unwrap()
                            .unwrap(),
                        port,
                    )
                })
                .map(|(index, port)| OutputPort::new(index, port));

            self.runtimes = std::iter::from_fn(|| {
                let mut runtime = base_rt.clone();
                let ext_in_handle = runtime.extern_inputs().get("Midi").unwrap().to_bits();
                Some(VoiceRuntime {
                    runtime,
                    ext_in_handle,
                })
            })
            .take(voice_count)
            .collect::<Vec<_>>();
            self.splitter.voices = vec![None; voice_count];
        }

        if let Some((channel, msg)) = self.midi_in.pop_msg(&data[0]) {
            self.splitter.route(msg, |voice, message| {
                let VoiceRuntime {
                    runtime,
                    ext_in_handle,
                } = &mut self.runtimes[voice];
                runtime.extern_inputs().extend(
                    Index::from_bits(*ext_in_handle).unwrap(),
                    std::iter::once(Value::Midi { channel, message }),
                );
            });
        }

        let factor = 1.0 / (voice_count as f32);
        self.out = 0.0;
        if let Some(playback_port) = self.playback_port {
            for VoiceRuntime { runtime, .. } in &mut self.runtimes {
                let _events = runtime.step();
                self.out += runtime.peek(playback_port).as_float().unwrap_or_default() * factor;
            }
        }

        Default::default()
    }

    fn config(&self) -> Option<Arc<dyn NodeConfig>> {
        Some(self.conf.clone())
    }

    fn inputs(&self) -> Vec<Input> {
        vec![Input::stateful("midi", &self.midi_in)]
    }

    fn read(&self, out: &mut [Value]) {
        out[0] = Value::Float(self.out);
    }
}

pub fn polyphonic() -> Box<dyn Node> {
    Box::new(PolyphonicInstrument {
        conf: Arc::new(PolyphonicInstrumentConf {
            editor: Mutex::new(LazyEditor::Pending(None)),
            editor_notified: AtomicBool::new(false),
            voices: AtomicU8::new(4),
            topology_changed: Mutex::new(None),
        }),
        template_reader: None,
        template_reader_queue: 0,
        midi_in: Arc::new(MidiInput::new()),
        runtimes: Vec::new(),
        splitter: MidiSplitter::new(4),
        playback_port: None,
        out: 0.0,
    })
}
