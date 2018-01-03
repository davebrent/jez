use std::io::Error;
use std::net::UdpSocket;
use std::sync::mpsc::Receiver;
use std::thread;

use rosc::{OscMessage, OscPacket, OscType};
use rosc::encoder;

use err::SysErr;
use vm::Command;

impl From<Error> for SysErr {
    fn from(_: Error) -> SysErr {
        SysErr::UnreachableBackend
    }
}

fn dispatch(sock: &UdpSocket, msg: Command) {
    match msg {
        Command::MidiNoteOn(chn, pitch, vel) => {
            let buff = encoder::encode(&OscPacket::Message(OscMessage {
                addr: "/note_on".to_string(),
                args: Some(vec![
                    OscType::Int(i32::from(chn)),
                    OscType::Int(i32::from(pitch)),
                    OscType::Int(i32::from(vel)),
                ]),
            })).unwrap();

            sock.send(&buff).unwrap();
        }
        Command::MidiNoteOff(chn, pitch) => {
            let buff = encoder::encode(&OscPacket::Message(OscMessage {
                addr: "/note_off".to_string(),
                args: Some(vec![
                    OscType::Int(i32::from(chn)),
                    OscType::Int(i32::from(pitch)),
                ]),
            })).unwrap();
            sock.send(&buff).unwrap();
        }
        Command::MidiCtl(chn, ctl, val) => {
            let buff = encoder::encode(&OscPacket::Message(OscMessage {
                addr: "/ctrl".to_string(),
                args: Some(vec![
                    OscType::Int(i32::from(chn)),
                    OscType::Int(i32::from(ctl)),
                    OscType::Int(i32::from(val)),
                ]),
            })).unwrap();
            sock.send(&buff).unwrap();
        }
        _ => (),
    }
}

pub struct Osc;

impl Osc {
    pub fn new(channel: Receiver<Command>) -> Result<Self, SysErr> {
        let sock = try!(UdpSocket::bind("127.0.0.1:34254"));
        try!(sock.connect("127.0.0.1:3000"));
        thread::spawn(move || while let Ok(msg) = channel.recv() {
            dispatch(&sock, msg);
        });
        Ok(Osc {})
    }
}
