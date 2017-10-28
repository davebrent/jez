use std::io::Error;
use std::net::UdpSocket;
use std::sync::mpsc::Receiver;
use std::thread;

use rosc::{OscMessage, OscPacket, OscType};
use rosc::encoder;

use err::SysErr;
use memory::RingBuffer;
use vm::{AudioBlock, Command};

impl From<Error> for SysErr {
    fn from(_: Error) -> SysErr {
        SysErr::UnreachableBackend
    }
}

pub struct Osc;

impl Osc {
    pub fn new(_: RingBuffer<AudioBlock>,
               channel: Receiver<Command>)
               -> Result<Self, SysErr> {
        let sock = try!(UdpSocket::bind("127.0.0.1:34254"));
        try!(sock.connect("127.0.0.1:3000"));

        thread::spawn(move || while let Ok(msg) = channel.recv() {
            match msg {
                Command::MidiNoteOn(chn, pitch, vel) => {
                    let buff =
                        encoder::encode(&OscPacket::Message(OscMessage {
                            addr: "/note_on".to_string(),
                            args: Some(vec![
                                OscType::Int(chn as i32),
                                OscType::Int(pitch as i32),
                                OscType::Int(vel as i32),
                            ]),
                        })).unwrap();
                    sock.send(&buff).unwrap();
                }
                Command::MidiNoteOff(chn, pitch) => {
                    let buff =
                        encoder::encode(&OscPacket::Message(OscMessage {
                            addr: "/note_off".to_string(),
                            args: Some(vec![
                                OscType::Int(chn as i32),
                                OscType::Int(pitch as i32),
                            ]),
                        })).unwrap();
                    sock.send(&buff).unwrap();
                }
                Command::MidiCtl(chn, ctl, val) => {
                    let buff =
                        encoder::encode(&OscPacket::Message(OscMessage {
                            addr: "/ctrl".to_string(),
                            args: Some(vec![
                                OscType::Int(chn as i32),
                                OscType::Int(ctl as i32),
                                OscType::Int(val as i32),
                            ]),
                        })).unwrap();
                    sock.send(&buff).unwrap();
                }
                _ => (),
            }
        });

        Ok(Osc {})
    }
}
