mod stream_audio_out;
mod synth_app;

use std::{
    collections::VecDeque,
    fmt::Debug,
    num::NonZeroU32,
    sync::{mpsc::Sender, Arc, LazyLock, Mutex},
};

use midly::{num::u7, MidiMessage};
use modal_lib::{
    compute::nodes::all::source::{MidiSource, MidiSourceNew},
    graph::MidiCollection,
    remote::{ExternInput, RtRequest},
};
use nih_plug::{
    midi::{MidiConfig, NoteEvent},
    nih_export_clap, nih_export_vst3,
    params::Params,
    plugin::Plugin,
    prelude::{
        AsyncExecutor, AudioIOLayout, AuxiliaryBuffers, Buffer, ClapFeature, ClapPlugin, Editor,
        ProcessContext, ProcessStatus, Vst3Plugin, Vst3SubCategory,
    },
};
use nih_plug_egui::{create_egui_editor, EguiState};
use runtime::{Value, ValueKind};
use serde::{Deserialize, Serialize};
use stream_audio_out::StreamReader;
use synth_app::SynthApp;

struct DawMidi {
    tx: barrage::Sender<(u8, MidiMessage)>,
    rx: barrage::Receiver<(u8, MidiMessage)>,
}

impl DawMidi {
    fn new() -> Self {
        let (tx, rx) = barrage::unbounded();
        DawMidi { tx, rx }
    }
}

static DAW_MIDI: LazyLock<DawMidi> = LazyLock::new(DawMidi::new);

#[derive(Clone, Debug, Serialize, Deserialize)]
struct DawMidiStreamNew;

#[typetag::serde]
impl MidiSourceNew for DawMidiStreamNew {
    fn name(&self) -> String {
        String::from("DAW")
    }

    fn new_src(&self) -> anyhow::Result<Box<dyn MidiSource>> {
        let src = DawMidiSource(DAW_MIDI.rx.clone());
        Ok(Box::new(src))
    }
}

struct DawMidiSource(barrage::Receiver<(u8, MidiMessage)>);

impl Debug for DawMidiSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DawMidiSource").finish()
    }
}

impl MidiSource for DawMidiSource {
    fn try_next(&mut self) -> Option<(u8, MidiMessage)> {
        self.0.try_recv().unwrap().map(|msg| dbg!(msg))
    }

    fn reset(&mut self) {}
}

pub struct Modal {
    app: Arc<Mutex<SynthApp>>,
    sender: Sender<RtRequest>,
    reader: StreamReader,
    params: Arc<ModalParams>,
    samples: VecDeque<f32>,
}

impl Default for Modal {
    fn default() -> Self {
        let (mut app, reader, sender) = SynthApp::new(None);
        app.user_state.ctx.midi.insert(
            "Track".to_string(),
            MidiCollection::Single(Box::new(DawMidiStreamNew)),
        );
        sender
            .send(RtRequest::ExternDefine {
                input: ExternInput::TrackAudio,
                kind: ValueKind::Float,
            })
            .ok();
        Modal {
            app: Arc::new(Mutex::new(app)),
            sender,
            reader,
            params: Arc::new(ModalParams::default()),
            samples: VecDeque::default(),
        }
    }
}

#[derive(Params)]
pub struct ModalParams {
    /// The editor state, saved together with the parameter state so the custom scaling can be
    /// restored.
    #[persist = "editor-state"]
    editor_state: Arc<EguiState>,
}

impl Default for ModalParams {
    fn default() -> Self {
        Self {
            editor_state: EguiState::from_size(1280, 720),
        }
    }
}

impl Plugin for Modal {
    const NAME: &'static str = "Modal";
    const VENDOR: &'static str = "Kamil Koczurek";
    const URL: &'static str = "https://github.com/kamirr/modal";
    const EMAIL: &'static str = "koczurekk@gmail.com";
    const VERSION: &'static str = "0.1.0";

    const MIDI_INPUT: MidiConfig = MidiConfig::Basic;

    const AUDIO_IO_LAYOUTS: &'static [AudioIOLayout] = &[AudioIOLayout {
        main_input_channels: NonZeroU32::new(1),
        main_output_channels: NonZeroU32::new(1),
        ..AudioIOLayout::const_default()
    }];

    type SysExMessage = ();

    type BackgroundTask = ();

    fn params(&self) -> Arc<dyn Params> {
        self.params.clone()
    }

    fn editor(&mut self, _executor: AsyncExecutor<Self>) -> Option<Box<dyn Editor>> {
        let app = Arc::clone(&self.app);
        create_egui_editor(
            self.params.editor_state.clone(),
            (),
            move |_, _| {},
            move |egui_ctx, _setter, _state| {
                app.lock().unwrap().update(egui_ctx);
            },
        )
    }

    fn process(
        &mut self,
        buffer: &mut Buffer,
        _aux: &mut AuxiliaryBuffers,
        context: &mut impl ProcessContext<Self>,
    ) -> ProcessStatus {
        if let Some(ev) = context.next_event() {
            let mut midi_msg = None;
            match ev {
                NoteEvent::NoteOn {
                    channel,
                    note,
                    velocity,
                    ..
                } => {
                    midi_msg = Some((
                        channel,
                        MidiMessage::NoteOn {
                            key: u7::from_int_lossy(note),
                            vel: u7::from_int_lossy((velocity * 127.0) as u8),
                        },
                    ))
                }
                NoteEvent::NoteOff {
                    channel,
                    note,
                    velocity,
                    ..
                } => {
                    midi_msg = Some((
                        channel,
                        MidiMessage::NoteOff {
                            key: u7::from_int_lossy(note),
                            vel: u7::from_int_lossy((velocity * 127.0) as u8),
                        },
                    ))
                }

                other => {
                    println!("Unsupported event: {other:?}");
                }
            }

            if let Some(msg) = midi_msg {
                DAW_MIDI.tx.send(msg).unwrap();
            }
        }

        let samples = buffer
            .iter_samples()
            .map(|mut s| *s.get_mut(0).unwrap())
            .map(Value::Float)
            .collect::<Vec<_>>();

        let samples_len = samples.len();

        self.sender
            .send(RtRequest::ExternAppend {
                input: ExternInput::TrackAudio,
                values: samples,
            })
            .ok();

        while self.samples.len() < samples_len {
            let chunk = self.reader.read();
            self.samples.extend(chunk.into_iter());
        }

        for mut sample in buffer.iter_samples() {
            *sample.get_mut(0).unwrap() = self.samples.pop_front().unwrap_or_default();
        }

        ProcessStatus::Normal
    }
}

impl ClapPlugin for Modal {
    const CLAP_ID: &'static str = "com.kamil.koczurek.modal";
    const CLAP_DESCRIPTION: Option<&'static str> = Some("Modular Music Synthesiser");
    const CLAP_MANUAL_URL: Option<&'static str> = Some(Self::URL);
    const CLAP_SUPPORT_URL: Option<&'static str> = None;
    const CLAP_FEATURES: &'static [ClapFeature] = &[
        ClapFeature::AudioEffect,
        ClapFeature::Mono,
        ClapFeature::Synthesizer,
        ClapFeature::Instrument,
    ];
}

impl Vst3Plugin for Modal {
    const VST3_CLASS_ID: [u8; 16] = *b"ModalSynth0xBEEF";
    const VST3_SUBCATEGORIES: &'static [Vst3SubCategory] = &[
        Vst3SubCategory::Fx,
        Vst3SubCategory::Mono,
        Vst3SubCategory::Synth,
        Vst3SubCategory::Instrument,
    ];
}

nih_export_clap!(Modal);
nih_export_vst3!(Modal);
