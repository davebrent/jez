use rosc::encoder;
use rosc::{OscMessage, OscPacket, OscType};

use std::net::UdpSocket;

use crate::err::Error;
use crate::sinks::sink::Sink;
use crate::vm::Command;

pub struct Renoise {
    sock: UdpSocket,
}

impl Renoise {
    pub fn new(host_addr: &str, client_addr: &str) -> Result<Self, Error> {
        let sock = UdpSocket::bind(host_addr)?;
        sock.connect(client_addr)?;
        Ok(Renoise { sock: sock })
    }
}

impl Sink for Renoise {
    fn name(&self) -> &str {
        "renoise"
    }

    fn process(&mut self, cmd: Command) {
        if let Some(buff) = encode(cmd) {
            self.sock.send(&buff).unwrap();
        }
    }
}

fn encode(cmd: Command) -> Option<Vec<u8>> {
    println!("{:?}", cmd);
    match cmd {
        Command::MidiNoteOn(chn, pitch, vel) => {
            Some(
                encoder::encode(&OscPacket::Message(OscMessage {
                    addr: "/renoise/trigger/note_on".to_string(),
                    args: vec![
                        // Instrument
                        OscType::Int(i32::from(chn + 1)),
                        // Track (current one)
                        OscType::Int(i32::from(chn + 1)),
                        // Pitch
                        OscType::Int(i32::from(pitch)),
                        // Velocity
                        OscType::Int(i32::from(vel)),
                    ],
                }))
                .unwrap(),
            )
        }
        Command::MidiNoteOff(chn, pitch) => Some(
            encoder::encode(&OscPacket::Message(OscMessage {
                addr: "/renoise/trigger/note_off".to_string(),
                args: vec![
                    // Instrument
                    OscType::Int(i32::from(chn + 1)),
                    // Track (current one)
                    OscType::Int(i32::from(chn + 1)),
                    // Pitch
                    OscType::Int(i32::from(pitch)),
                ],
            }))
            .unwrap(),
        ),
        _ => None,
    }
}
