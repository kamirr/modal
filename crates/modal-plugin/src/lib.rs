mod stream_audio_out;
mod synth_app;

use std::{
    collections::VecDeque,
    fmt::Debug,
    num::NonZeroU32,
    sync::{Arc, LazyLock, Mutex},
};

use midly::{num::u7, MidiMessage};
use modal_lib::compute::nodes::all::source::{MidiSource, MidiSourceNew};
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
use serde::{Deserialize, Serialize};
use stream_audio_out::StreamReader;
use synth_app::SynthApp;

static DAW_MIDI: LazyLock<(
    barrage::Sender<(u8, MidiMessage)>,
    barrage::Receiver<(u8, MidiMessage)>,
)> = LazyLock::new(|| barrage::unbounded());

#[derive(Clone, Debug, Serialize, Deserialize)]
struct DawMidiStreamNew;

#[typetag::serde]
impl MidiSourceNew for DawMidiStreamNew {
    fn name(&self) -> String {
        String::from("DAW")
    }

    fn new_src(&self) -> anyhow::Result<Box<dyn MidiSource>> {
        let src = DawMidiSource(DAW_MIDI.1.clone());
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
        if let Some(msg) = self.0.try_recv().unwrap() {
            Some(dbg!(msg))
        } else {
            None
        }
    }

    fn reset(&mut self) {}
}

pub struct Modal {
    app: Arc<Mutex<SynthApp>>,
    reader: StreamReader,
    params: Arc<ModalParams>,
    samples: VecDeque<f32>,
}

impl Default for Modal {
    fn default() -> Self {
        let (mut app, reader) = SynthApp::new(None);
        app.user_state
            .ctx
            .midi
            .insert("DAW".to_string(), vec![Box::new(DawMidiStreamNew)]);
        Modal {
            app: Arc::new(Mutex::new(app)),
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
            editor_state: EguiState::from_size(1920, 1080),
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
                println!("Received message: {msg:?}");
                DAW_MIDI.0.send(msg).unwrap();
            }
        }

        for channel in buffer.iter_samples() {
            while self.samples.len() < channel.len() {
                let chunk = self.reader.read();
                self.samples.extend(chunk.into_iter());
            }

            for sample in channel {
                *sample = self.samples.pop_front().unwrap_or_default();
            }
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
        ClapFeature::Utility,
    ];
}

impl Vst3Plugin for Modal {
    const VST3_CLASS_ID: [u8; 16] = *b"ModalSynth0xBEEF";
    const VST3_SUBCATEGORIES: &'static [Vst3SubCategory] =
        &[Vst3SubCategory::Fx, Vst3SubCategory::Tools];
}

nih_export_clap!(Modal);
nih_export_vst3!(Modal);
