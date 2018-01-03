use std::convert::From;
use std::default::Default;
use std::sync::mpsc::Receiver;

use jack::prelude::{AsyncClient, Client, JackControl, JackErr, MidiOutPort,
                    MidiOutSpec, NotificationHandler, Port, PortSpec,
                    ProcessHandler, ProcessScope, RawMidi, client_options};

use err::SysErr;
use vm::Command;

struct Notifier;
struct Processor {
    channel: Receiver<Command>,
    midi_out: Vec<Port<MidiOutSpec>>,
}

pub struct Jack {
    _client: AsyncClient<Notifier, Processor>,
}

impl From<JackErr> for SysErr {
    fn from(_: JackErr) -> SysErr {
        SysErr::UnreachableBackend
    }
}

fn make_ports<T>(prefix: &'static str,
                 client: &Client,
                 len: usize)
                 -> Result<Vec<Port<T>>, SysErr>
where
    T: PortSpec + Default,
{
    let mut ports = Vec::with_capacity(len);
    for i in 1..len + 1 {
        let name = format!("{}_{}", prefix, i);
        match client.register_port(name.as_str(), Default::default()) {
            Ok(port) => ports.push(port),
            Err(_) => return Err(SysErr::UnreachableBackend),
        };
    }
    Ok(ports)
}

impl NotificationHandler for Notifier {}

impl Processor {
    fn process_msgs(&mut self,
                    _: &Client,
                    ps: &ProcessScope)
                    -> Result<(), SysErr> {
        let mut ports: Vec<MidiOutPort> = self.midi_out
            .iter_mut()
            .map(|port| MidiOutPort::new(port, ps))
            .collect();

        while let Ok(msg) = self.channel.try_recv() {
            match msg {
                Command::MidiNoteOn(chn, pitch, vel) => {
                    let midi = RawMidi {
                        time: 0,
                        bytes: &[144 + chn, pitch, vel],
                    };
                    ports[0].write(&midi).unwrap();
                }
                Command::MidiNoteOff(chn, pitch) => {
                    let midi = RawMidi {
                        time: 0,
                        bytes: &[128 + chn, pitch, 0],
                    };
                    ports[0].write(&midi).unwrap();
                }
                Command::MidiCtl(chn, ctl, val) => {
                    let midi = RawMidi {
                        time: 0,
                        bytes: &[176 + chn, ctl, val],
                    };
                    ports[0].write(&midi).unwrap();
                }
                _ => (),
            }
        }

        Ok(())
    }
}

impl ProcessHandler for Processor {
    fn process(&mut self, client: &Client, ps: &ProcessScope) -> JackControl {
        match self.process_msgs(client, ps) {
            Ok(_) => JackControl::Continue,
            Err(_) => JackControl::Quit,
        }
    }
}

impl Jack {
    pub fn new(channel: Receiver<Command>) -> Result<Self, SysErr> {
        let opts = client_options::NO_START_SERVER;
        let (client, _) = try!(Client::new("jez", opts));
        let notifier = Notifier {};
        let processor = Processor {
            channel: channel,
            midi_out: try!(make_ports("midiout", &client, 1)),
        };

        Ok(Jack {
            _client: try!(AsyncClient::new(client, notifier, processor)),
        })
    }
}
