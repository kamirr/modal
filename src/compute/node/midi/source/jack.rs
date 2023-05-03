use anyhow::{anyhow, Result};
use jack::{ClientOptions, PortFlags, PortSpec};
use midly::{live::LiveEvent, Arena, MidiMessage, TrackEventKind};
use serde::{Deserialize, Serialize};
use std::{
    any::Any,
    sync::mpsc::{channel, Receiver},
};

use super::{MidiSource, MidiSourceNew};

#[derive(Debug)]
pub struct JackSource {
    // the type is hard to spell and it only needs to be kept alive.
    _client: Box<dyn Any + Send + Sync>,
    midi_rx: Receiver<(u8, MidiMessage)>,
}

impl MidiSource for JackSource {
    fn try_next(&mut self) -> Option<(u8, MidiMessage)> {
        self.midi_rx.try_recv().ok()
    }

    fn reset(&mut self) {}
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JackSourceNew {
    port_name: String,
}

impl JackSourceNew {
    pub fn all() -> Vec<JackSourceNew> {
        let Ok((client, _status)) = jack::Client::new(
            &format!("modal-synth-tmp"),
            ClientOptions::NO_START_SERVER,
        ) else {
            return Default::default()
        };

        let port_names = client.ports(
            None,
            Some(jack::MidiOut.jack_port_type()),
            PortFlags::IS_OUTPUT,
        );

        port_names
            .into_iter()
            .map(|port_name| JackSourceNew { port_name })
            .collect()
    }
}

#[typetag::serde]
impl MidiSourceNew for JackSourceNew {
    fn new_src(&self) -> Result<Box<dyn MidiSource>> {
        let (client, _status) = jack::Client::new(
            &format!("modal-synth-{:x}", rand::random::<u32>()),
            ClientOptions::NO_START_SERVER,
        )?;

        let midi_in = client.register_port("midi-in", jack::MidiIn::default())?;
        let midi_in2 = midi_in.clone_unowned();

        let midi_out = client
            .port_by_name(&self.port_name)
            .ok_or(anyhow!("Port doesn't exist"))?;

        let (midi_tx, midi_rx) = channel();
        let mut arena = Arena::new();
        let process_cb = move |_: &jack::Client, ps: &jack::ProcessScope| {
            for msg in midi_in.iter(ps) {
                if let Ok(live_ev) = LiveEvent::parse(msg.bytes) {
                    let track_ev = live_ev.as_track_event(&mut arena);

                    if let TrackEventKind::Midi { channel, message } = track_ev {
                        midi_tx.send((channel.as_int(), message)).ok();
                    }
                }
            }

            jack::Control::Continue
        };

        let async_client =
            client.activate_async((), jack::ClosureProcessHandler::new(process_cb))?;

        async_client
            .as_client()
            .connect_ports(&midi_out, &midi_in2)?;

        Ok(Box::new(JackSource {
            _client: Box::new(async_client),
            midi_rx,
        }))
    }

    fn name(&self) -> String {
        self.port_name.clone()
    }
}
