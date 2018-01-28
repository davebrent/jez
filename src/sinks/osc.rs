use std::io::Error;
use std::net::UdpSocket;

use rosc::{OscMessage, OscPacket, OscType};
use rosc::encoder;

use err::SysErr;
use vm::Command;

use super::sink::Sink;


impl From<Error> for SysErr {
    fn from(_: Error) -> SysErr {
        SysErr::UnreachableBackend
    }
}

pub struct Osc {
    sock: UdpSocket,
}

impl Osc {
    pub fn new(host_addr: &str, client_addr: &str) -> Result<Self, SysErr> {
        let sock = try!(UdpSocket::bind(host_addr));
        try!(sock.connect(client_addr));
        Ok(Osc { sock: sock })
    }
}

impl Sink for Osc {
    fn name(&self) -> &str {
        "osc"
    }

    fn recieve(&mut self, cmd: Command) {
        let buff = match cmd {
            Command::MidiNoteOn(chn, pitch, vel) => {
                encoder::encode(&OscPacket::Message(OscMessage {
                    addr: "/note_on".to_string(),
                    args: Some(vec![
                        OscType::Int(i32::from(chn)),
                        OscType::Int(i32::from(pitch)),
                        OscType::Int(i32::from(vel)),
                    ]),
                })).unwrap()
            }
            Command::MidiNoteOff(chn, pitch) => {
                encoder::encode(&OscPacket::Message(OscMessage {
                    addr: "/note_off".to_string(),
                    args: Some(vec![
                        OscType::Int(i32::from(chn)),
                        OscType::Int(i32::from(pitch)),
                    ]),
                })).unwrap()
            }
            Command::MidiCtl(chn, ctl, val) => {
                encoder::encode(&OscPacket::Message(OscMessage {
                    addr: "/ctrl".to_string(),
                    args: Some(vec![
                        OscType::Int(i32::from(chn)),
                        OscType::Int(i32::from(ctl)),
                        OscType::Int(i32::from(val)),
                    ]),
                })).unwrap()
            }
            _ => return,
        };

        self.sock.send(&buff).unwrap();
    }
}
