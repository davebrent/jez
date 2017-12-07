use std::sync::mpsc::Receiver;
use std::thread;

use err::SysErr;
use vm::{AudioBlock, Command, RingBuffer};

use portmidi as pm;

impl From<pm::Error> for SysErr {
    fn from(_: pm::Error) -> SysErr {
        SysErr::UnreachableBackend
    }
}

fn dispatch(port: &mut pm::OutputPort, msg: Command) {
    match msg {
        Command::MidiNoteOn(chn, pitch, vel) => {
            let msg = pm::MidiMessage {
                status: 144 + chn,
                data1: pitch,
                data2: vel,
            };
            port.write_message(msg).unwrap();
        }
        Command::MidiNoteOff(chn, pitch) => {
            let msg = pm::MidiMessage {
                status: 128 + chn,
                data1: pitch,
                data2: 0,
            };
            port.write_message(msg).unwrap();
        }
        Command::MidiCtl(chn, ctl, val) => {
            let msg = pm::MidiMessage {
                status: 176 + chn,
                data1: ctl,
                data2: val,
            };
            port.write_message(msg).unwrap();
        }
        _ => (),
    }
}

pub struct Portmidi;

impl Portmidi {
    pub fn new(_: RingBuffer<AudioBlock>,
               channel: Receiver<Command>)
               -> Result<Self, SysErr> {
        thread::spawn(move || {
            let context = pm::PortMidi::new().unwrap();

            let id = context.default_output_device_id().unwrap();
            let info = context.device(id).unwrap();
            let mut port = context.output_port(info, 1024).unwrap();

            while let Ok(msg) = channel.recv() {
                dispatch(&mut port, msg);
            }
        });

        Ok(Portmidi {})
    }
}
