use std::fmt;

use portmidi as pm;

use err::SysErr;
use vm::Command;

use super::sink::{Device, Sink};

impl From<pm::Error> for SysErr {
    fn from(_: pm::Error) -> SysErr {
        SysErr::UnreachableBackend
    }
}

pub struct Portmidi {
    ctx: pm::PortMidi,
    port: Option<pm::OutputPort>,
}

pub struct PortmidiDevice {
    dev: pm::DeviceInfo,
}

impl fmt::Display for PortmidiDevice {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.dev)
    }
}

impl Device for PortmidiDevice {}

impl Portmidi {
    pub fn new(id: Option<usize>) -> Result<Self, SysErr> {
        let ctx = try!(pm::PortMidi::new());

        let id = match id {
            Some(id) => Some(id as i32),
            None => match ctx.default_output_device_id() {
                Ok(id) => Some(id),
                Err(_) => None,
            },
        };

        let port = match id {
            Some(id) => {
                let info = try!(ctx.device(id));
                Some(try!(ctx.output_port(info, 1024)))
            }
            None => None,
        };

        Ok(Portmidi {
            ctx: ctx,
            port: port,
        })
    }
}

unsafe impl Send for Portmidi {}
unsafe impl Sync for Portmidi {}

impl Sink for Portmidi {
    fn name(&self) -> &str {
        "portmidi"
    }

    fn devices(&self) -> Vec<Box<Device>> {
        let mut devices: Vec<Box<Device>> = vec![];
        for dev in self.ctx.devices().unwrap() {
            devices.push(Box::new(PortmidiDevice { dev: dev.clone() }));
        }
        devices
    }

    fn recieve(&mut self, cmd: Command) {
        let msg = match cmd {
            Command::MidiNoteOn(chn, pitch, vel) => pm::MidiMessage {
                status: 144 + chn,
                data1: pitch,
                data2: vel,
            },
            Command::MidiNoteOff(chn, pitch) => pm::MidiMessage {
                status: 128 + chn,
                data1: pitch,
                data2: 0,
            },
            Command::MidiCtl(chn, ctl, val) => pm::MidiMessage {
                status: 176 + chn,
                data1: ctl,
                data2: val,
            },
            _ => return,
        };

        match self.port {
            Some(ref mut port) => {
                port.write_message(msg).unwrap();
            }
            _ => (),
        }
    }
}
