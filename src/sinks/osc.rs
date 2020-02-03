use rosc::encoder;
use rosc::{OscMessage, OscPacket, OscType};

use crate::vm::Command;

pub fn encode(cmd: Command) -> Option<Vec<u8>> {
    match cmd {
        Command::MidiNoteOn(chn, pitch, vel) => Some(
            encoder::encode(&OscPacket::Message(OscMessage {
                addr: "/note_on".to_string(),
                args: vec![
                    OscType::Int(i32::from(chn)),
                    OscType::Int(i32::from(pitch)),
                    OscType::Int(i32::from(vel)),
                ],
            }))
            .unwrap(),
        ),
        Command::MidiNoteOff(chn, pitch) => Some(
            encoder::encode(&OscPacket::Message(OscMessage {
                addr: "/note_off".to_string(),
                args: vec![OscType::Int(i32::from(chn)), OscType::Int(i32::from(pitch))],
            }))
            .unwrap(),
        ),
        Command::MidiCtl(chn, ctl, val) => Some(
            encoder::encode(&OscPacket::Message(OscMessage {
                addr: "/ctrl".to_string(),
                args: vec![
                    OscType::Int(i32::from(chn)),
                    OscType::Int(i32::from(ctl)),
                    OscType::Int(i32::from(val)),
                ],
            }))
            .unwrap(),
        ),
        _ => None,
    }
}
