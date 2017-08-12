use std::sync::mpsc::Receiver;
use std::time::Instant;

use err::SysErr;
use log::Logger;
use unit::Command;
use jack::prelude::{AsyncClient, Client, client_options, JackControl,
                    MidiOutPort, MidiOutSpec, NotificationHandler,
                    ProcessHandler, ProcessScope, Port, RawMidi};


struct Notifier;
impl NotificationHandler for Notifier {}

struct Processor {
    channel: Receiver<Command>,
    midi_out_port: Port<MidiOutSpec>,
    logger: Logger,
    start: Instant,
}

impl ProcessHandler for Processor {
    fn process(&mut self, _: &Client, ps: &ProcessScope) -> JackControl {
        let mut out_port = MidiOutPort::new(&mut self.midi_out_port, ps);

        while let Ok(msg) = self.channel.try_recv() {
            let time = Instant::now() - self.start;
            self.logger.log(time, "backend", &msg);
            match msg {
                Command::MidiNoteOn(chn, pitch, vel) => {
                    let midi = RawMidi {
                        time: 0,
                        bytes: &[144 + chn, pitch, vel],
                    };
                    out_port.write(&midi).unwrap();
                }
                Command::MidiNoteOff(chn, pitch) => {
                    let midi = RawMidi {
                        time: 0,
                        bytes: &[128 + chn, pitch, 0],
                    };
                    out_port.write(&midi).unwrap();
                }
                Command::MidiCtl(chn, ctl, val) => {
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
    pub fn new(logger: Logger,
               channel: Receiver<Command>)
               -> Result<Self, SysErr> {
        match Client::new("jez", client_options::NO_START_SERVER) {
            Err(_) => Err(SysErr::UnreachableBackend),
            Ok((client, _)) => {
                let midi_out_port = client
                    .register_port("midiout_1", MidiOutSpec::default())
                    .unwrap();

                let notifier = Notifier {};
                let processor = Processor {
                    channel: channel,
                    midi_out_port: midi_out_port,
                    logger: logger,
                    start: Instant::now(),
                };

                let ac = AsyncClient::new(client, notifier, processor).unwrap();
                Ok(Jack { _active_client: ac })
            }
        }
    }
}
