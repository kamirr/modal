use std::{
    collections::HashMap,
    time::{Duration, Instant},
};

use midly::{num::u7, MetaMessage, MidiMessage, Smf, Timing, TrackEventKind};

#[derive(Clone, Copy, Debug)]
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

#[derive(Clone, Debug)]
struct Controller {
    values: HashMap<u7, u7>,
}

impl Controller {
    fn new() -> Self {
        Controller {
            values: HashMap::new(),
        }
    }

    fn update(&mut self, message: &MidiMessage) {
        match message {
            MidiMessage::Controller { controller, value } => {
                self.values.insert(*controller, *value);
            }
            _ => {}
        }
    }
}

#[derive(Clone, Copy, Debug)]
struct MonoNote {
    key: u7,
    vel: u7,
}

impl MonoNote {
    fn new() -> Self {
        MonoNote {
            key: 0.into(),
            vel: 0.into(),
        }
    }

    fn update(&mut self, message: &MidiMessage) {
        match message {
            MidiMessage::NoteOn { key, vel } => {
                self.key = *key;
                self.vel = *vel;
            }
            MidiMessage::NoteOff { key, .. } => {
                if key == &self.key {
                    self.vel = 0.into()
                }
            }
            _ => {}
        }
    }
}

#[derive(Clone, Debug)]
pub struct Instrument {
    mono_note: MonoNote,
    controller: Controller,
}

impl Instrument {
    fn new() -> Self {
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
        let key = self.mono_note.key.as_int() as f32;
        440.0 * 2f32.powf((key - 69.0) / 12.0)
    }

    pub fn vel(&self) -> f32 {
        self.mono_note.vel.as_int() as f32 / 127.0
    }
}

#[derive(Clone, Debug)]
struct MidiState {
    tick: Duration,
    instruments: Vec<Instrument>,
}

#[derive(Clone, Debug)]
pub struct MidiPlayback {
    state: MidiState,
    smf: Smf<'static>,
    cursors: Vec<usize>,
    last_ev_tick: Vec<u32>,
    t0: Instant,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PlaybackStepResponse {
    Idle,
    Finished,
    MadeProgress,
}

impl MidiPlayback {
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

        let state = MidiState { tick, instruments };

        MidiPlayback {
            state,
            smf,
            cursors,
            last_ev_tick,
            t0: Instant::now(),
        }
    }

    pub fn start(&mut self) {
        self.cursors = std::iter::repeat(0).take(self.smf.tracks.len()).collect();
        self.last_ev_tick = std::iter::repeat(0).take(self.smf.tracks.len()).collect();
        self.t0 = Instant::now();
    }

    pub fn step(&mut self) -> PlaybackStepResponse {
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

    pub fn tracks(&self) -> u32 {
        self.smf.tracks.len() as _
    }

    pub fn instrument(&self, track: u32) -> &Instrument {
        let idx = track as usize;
        &self.state.instruments[idx]
    }
}
