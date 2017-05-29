use std::sync::mpsc::Receiver;

use unit::{RuntimeErr, Message};

use jack::prelude::{AsyncClient, Client, client_options, JackControl,
                    MidiOutPort, MidiOutSpec, NotificationHandler,
                    ProcessHandler, ProcessScope, Port, RawMidi};

use super::base::Backend;


struct Notifier;
impl NotificationHandler for Notifier {}

struct Processor {
    channel: Receiver<Message>,
    midi_out_port: Port<MidiOutSpec>,
}

impl ProcessHandler for Processor {
    fn process(&mut self, _: &Client, ps: &ProcessScope) -> JackControl {
        let mut out_port = MidiOutPort::new(&mut self.midi_out_port, ps);

        while let Ok(msg) = self.channel.try_recv() {
            match msg {
                Message::MidiNoteOn(chn, pitch, vel) => {
                    let midi = RawMidi {
                        time: 0,
                        bytes: &[144 + chn, pitch, vel],
                    };
                    out_port.write(&midi).unwrap();
                }
                Message::MidiNoteOff(chn, pitch) => {
                    let midi = RawMidi {
                        time: 0,
                        bytes: &[128 + chn, pitch, 0],
                    };
                    out_port.write(&midi).unwrap();
                }
                Message::MidiCtl(chn, ctl, val) => {
                    let midi = RawMidi {
                        time: 0,
                        bytes: &[176 + chn, ctl, val],
                    };
                    out_port.write(&midi).unwrap();
                }
                _ => (),
            }
        }

        JackControl::Continue
    }
}

pub struct Jack {
    _active_client: AsyncClient<Notifier, Processor>,
}

impl Jack {
    pub fn new(channel: Receiver<Message>) -> Result<Self, RuntimeErr> {
        match Client::new("jez", client_options::NO_START_SERVER) {
            Err(_) => Err(RuntimeErr::BackendUnreachable),
            Ok((client, _)) => {
                let midi_out_port = client
                    .register_port("midiout_1", MidiOutSpec::default())
                    .unwrap();

                let notifier = Notifier {};
                let processor = Processor {
                    channel: channel,
                    midi_out_port: midi_out_port,
                };

                let ac = AsyncClient::new(client, notifier, processor).unwrap();
                Ok(Jack { _active_client: ac })
            }
        }
    }
}

impl Backend for Jack {
    fn drain(&mut self) {}
}
