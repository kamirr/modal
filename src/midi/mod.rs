use std::{
    collections::HashMap,
    fmt::Debug,
    sync::mpsc::Receiver,
    time::{Duration, Instant},
};

use midly::{MetaMessage, MidiMessage, Smf, Timing, TrackEventKind};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
struct Beat {
    us: u32,
}

impl Beat {
    fn us(&self) -> u32 {
        self.us
    }

    fn update(&mut self, message: &MetaMessage) {
        if let MetaMessage::Tempo(us) = message {
            self.us = us.as_int();
        }
    }
}

impl Default for Beat {
    fn default() -> Self {
        Beat { us: 666667 }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Controller {
    values: HashMap<u8, u8>,
}

impl Controller {
    pub fn new() -> Self {
        Controller {
            values: HashMap::new(),
        }
    }

    fn update(&mut self, message: &MidiMessage) {
        match message {
            MidiMessage::Controller { controller, value } => {
                self.values.insert(controller.as_int(), value.as_int());
            }
            _ => {}
        }
    }

    pub fn get_control(&self, ctrl: u32) -> Option<f32> {
        self.values.get(&(ctrl as u8)).map(|v| *v as f32 / 127.0)
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct MonoNote {
    key: u8,
    vel: u8,
}

impl MonoNote {
    fn new() -> Self {
        MonoNote { key: 0, vel: 0 }
    }

    fn update(&mut self, message: &MidiMessage) {
        match message {
            MidiMessage::NoteOn { key, vel } => {
                self.key = key.as_int();
                self.vel = vel.as_int();
            }
            MidiMessage::NoteOff { key, .. } => {
                if key == &self.key {
                    self.vel = 0;
                }
            }
            _ => {}
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Instrument {
    pub mono_note: MonoNote,
    pub controller: Controller,
}

impl Instrument {
    pub fn new() -> Self {
        Instrument {
            mono_note: MonoNote::new(),
            controller: Controller::new(),
        }
    }

    fn update(&mut self, message: &MidiMessage) {
        self.mono_note.update(message);
        self.controller.update(message);
    }

    pub fn freq(&self) -> f32 {
        let key = self.mono_note.key as f32;
        440.0 * 2f32.powf((key - 69.0) / 12.0)
    }

    pub fn vel(&self) -> f32 {
        self.mono_note.vel as f32 / 127.0
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum PlaybackStepResponse {
    Idle,
    Finished,
    MadeProgress,
}

#[typetag::serde]
pub trait MidiPlayback {
    fn start(&mut self);
    fn step(&mut self) -> PlaybackStepResponse;
    fn tracks(&self) -> u32;
    fn instrument(&self, track: u32) -> &Instrument;
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct SfmMidiState {
    tick: Duration,
    instruments: Vec<Instrument>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SmfMidiPlayback {
    state: SfmMidiState,
    #[serde(with = "crate::util::serde_smf")]
    smf: Smf<'static>,
    cursors: Vec<usize>,
    last_ev_tick: Vec<u32>,
    #[serde(skip)]
    #[serde(default = "Instant::now")]
    t0: Instant,
}

impl SmfMidiPlayback {
    pub fn new(smf: Smf<'static>) -> Self {
        let mut beat = Beat::default();
        for ev in &smf.tracks[0] {
            if let TrackEventKind::Meta(message) = &ev.kind {
                beat.update(message);
            }
        }

        let tick = Duration::from_secs_f64(match smf.header.timing {
            Timing::Metrical(ticks_per_beat) => {
                beat.us() as f64 / 1000000.0 / ticks_per_beat.as_int() as f64
            }
            Timing::Timecode(fps, subframe) => 1f64 / fps.as_f32() as f64 / subframe as f64,
        });

        let mut instruments = Vec::new();
        for _ in 0..smf.tracks.len() {
            instruments.push(Instrument::new());
        }

        let cursors = std::iter::repeat(0).take(smf.tracks.len()).collect();
        let last_ev_tick = std::iter::repeat(0).take(smf.tracks.len()).collect();

        let state = SfmMidiState { tick, instruments };

        SmfMidiPlayback {
            state,
            smf,
            cursors,
            last_ev_tick,
            t0: Instant::now(),
        }
    }
}

#[typetag::serde]
impl MidiPlayback for SmfMidiPlayback {
    fn start(&mut self) {
        self.cursors = std::iter::repeat(0).take(self.smf.tracks.len()).collect();
        self.last_ev_tick = std::iter::repeat(0).take(self.smf.tracks.len()).collect();
        self.t0 = Instant::now();
    }

    fn step(&mut self) -> PlaybackStepResponse {
        let t = Instant::now() - self.t0;
        let tick_f = t.as_secs_f64() / self.state.tick.as_secs_f64();
        let tick_n = tick_f.round() as u32;

        let mut response = PlaybackStepResponse::Finished;

        for (k, track) in self.smf.tracks.iter().enumerate() {
            let Some(next_ev) = track.get(self.cursors[k]) else {
                continue;
            };

            if response == PlaybackStepResponse::Finished {
                response = PlaybackStepResponse::Idle;
            }

            let target_tick = self.last_ev_tick[k] + next_ev.delta.as_int();

            if tick_n >= target_tick {
                if let TrackEventKind::Midi { message, .. } = &next_ev.kind {
                    self.state.instruments[k].update(message);
                }

                self.last_ev_tick[k] = target_tick;
                self.cursors[k] += 1;

                response = PlaybackStepResponse::MadeProgress;
            }
        }

        response
    }

    fn tracks(&self) -> u32 {
        self.smf.tracks.len() as _
    }

    fn instrument(&self, track: u32) -> &Instrument {
        let idx = track as usize;
        &self.state.instruments[idx]
    }
}

#[derive(Debug)]
struct JackMidiSource {
    midi_in: Receiver<TrackEventKind<'static>>,
    client: Option<crate::jack::Client>,
}

impl Default for JackMidiSource {
    fn default() -> Self {
        let mut client = crate::jack::Client::default();
        let midi_in = client.take_midi_stream().unwrap();

        JackMidiSource {
            midi_in,
            client: Some(client),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct JackMidiPlayback {
    #[serde(skip)]
    instruments: Vec<Instrument>,
    #[serde(skip)]
    midi_src: JackMidiSource,
}

impl JackMidiPlayback {
    pub fn new() -> Self {
        JackMidiPlayback {
            instruments: Default::default(),
            midi_src: Default::default(),
        }
    }

    pub fn client(&mut self) -> Option<crate::jack::Client> {
        self.midi_src.client.take()
    }
}

#[typetag::serde]
impl MidiPlayback for JackMidiPlayback {
    fn start(&mut self) {}

    fn step(&mut self) -> PlaybackStepResponse {
        while let Ok(ev) = self.midi_src.midi_in.try_recv() {
            if let TrackEventKind::Midi {
                channel, message, ..
            } = &ev
            {
                let channel = channel.as_int() as usize;

                while self.instruments.len() < channel + 1 {
                    self.instruments.push(Instrument::new());
                }

                self.instruments[channel].update(message);
            }
        }

        PlaybackStepResponse::MadeProgress
    }

    fn tracks(&self) -> u32 {
        self.instruments.len() as u32
    }

    fn instrument(&self, track: u32) -> &Instrument {
        &self.instruments[track as usize]
    }
}
