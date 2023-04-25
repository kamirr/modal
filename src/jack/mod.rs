use std::{
    any::Any,
    collections::VecDeque,
    sync::mpsc::{channel, Receiver, Sender},
};

use midly::{live::LiveEvent, Arena};

#[derive(Debug)]
pub struct Client {
    _inner: Box<dyn Any>,
    midi_rx: Option<Receiver<midly::TrackEventKind<'static>>>,
    audio_tx: Sender<f32>,
}

impl Client {
    pub fn take_midi_stream(&mut self) -> Option<Receiver<midly::TrackEventKind<'static>>> {
        self.midi_rx.take()
    }

    pub fn audio_out(&self) -> Sender<f32> {
        self.audio_tx.clone()
    }
}

impl Default for Client {
    fn default() -> Self {
        let (midi_tx, midi_rx) = channel();
        let (audio_tx, audio_rx) = channel::<f32>();

        let (client, _status) =
            jack::Client::new("modal-synth", jack::ClientOptions::NO_START_SERVER).unwrap();

        let midi_in = client
            .register_port("midi-in", jack::MidiIn::default())
            .unwrap();

        let mut audio_out = client
            .register_port("audio-out", jack::AudioOut::default())
            .unwrap();

        let mut arena = Arena::new();
        let mut samples = VecDeque::new();
        let process_cb = move |_: &jack::Client, ps: &jack::ProcessScope| {
            for msg in midi_in.iter(ps) {
                if let Ok(live_ev) = LiveEvent::parse(msg.bytes) {
                    let track_ev = live_ev.as_track_event(&mut arena).to_static();

                    midi_tx.send(track_ev).ok();
                }
            }

            let sample_buf = audio_out.as_mut_slice(ps);

            while samples.len() < sample_buf.len() {
                let Ok(next_sample) = audio_rx.try_recv() else {
                    break;
                };

                samples.push_back(next_sample);
            }

            for sample in sample_buf.iter_mut() {
                *sample = samples.pop_front().unwrap_or(0.0);
            }

            jack::Control::Continue
        };

        Client {
            _inner: Box::new(
                client.activate_async((), jack::ClosureProcessHandler::new(process_cb)),
            ),
            midi_rx: Some(midi_rx),
            audio_tx,
        }
    }
}
