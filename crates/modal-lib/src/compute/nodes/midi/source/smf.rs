use std::{
    collections::VecDeque,
    ffi::OsStr,
    fmt::Debug,
    path::Path,
    time::{Duration, Instant},
};

use anyhow::Result;

use midly::{MetaMessage, MidiMessage, Smf, Timing, TrackEventKind};
use serde::{Deserialize, Serialize};

use crate::compute::nodes::all::source::MidiSourceNew;

use super::MidiSource;

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
pub struct SmfSource {
    smf: Smf<'static>,
    cursors: Vec<usize>,
    last_ev_tick: Vec<u32>,
    t0: Instant,
    tick: Duration,
    queue: VecDeque<(u8, MidiMessage)>,
}

impl SmfSource {
    fn new(bytes: &[u8]) -> Result<Self> {
        let smf = Smf::parse(bytes)?.to_static();

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

        let cursors = std::iter::repeat(0).take(smf.tracks.len()).collect();
        let last_ev_tick = std::iter::repeat(0).take(smf.tracks.len()).collect();

        Ok(SmfSource {
            smf,
            cursors,
            last_ev_tick,
            t0: Instant::now(),
            tick,
            queue: VecDeque::new(),
        })
    }
}

impl MidiSource for SmfSource {
    fn try_next(&mut self) -> Option<(u8, MidiMessage)> {
        let t = Instant::now() - self.t0;
        let tick_f = t.as_secs_f64() / self.tick.as_secs_f64();
        let tick_n = tick_f.round() as u32;

        for (k, track) in self.smf.tracks.iter().enumerate() {
            let Some(next_ev) = track.get(self.cursors[k]) else {
                continue;
            };

            let target_tick = self.last_ev_tick[k] + next_ev.delta.as_int();

            if tick_n >= target_tick {
                if let TrackEventKind::Midi { channel, message } = next_ev.kind {
                    self.queue.push_back((channel.as_int(), message));
                }

                self.last_ev_tick[k] = target_tick;
                self.cursors[k] += 1;
            }
        }

        self.queue.pop_front()
    }

    fn reset(&mut self) {
        self.t0 = Instant::now();
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SmfSourceNew {
    bytes: Vec<u8>,
    name: String,
}

impl SmfSourceNew {
    pub fn new(path: &Path) -> Result<Self> {
        Ok(SmfSourceNew {
            bytes: std::fs::read(path)?,
            name: path
                .file_name()
                .map(OsStr::to_string_lossy)
                .map(|cow| cow.to_string())
                .unwrap_or_default(),
        })
    }
}

#[typetag::serde]
impl MidiSourceNew for SmfSourceNew {
    fn new_src(&self) -> Result<Box<dyn MidiSource>> {
        let src = SmfSource::new(&self.bytes)?;

        Ok(Box::new(src) as Box<_>)
    }

    fn name(&self) -> String {
        self.name.clone()
    }
}
