use std::sync::mpsc::Receiver;
use std::thread;
use std::time::Instant;

use math::dur_to_millis;
use unit::Message;

use super::base::Backend;


pub struct Debug;

impl Debug {
    pub fn new(channel: Receiver<Message>) -> Self {
        thread::spawn(move || {
            let start = Instant::now();
            while let Ok(msg) = channel.recv() {
                let t = dur_to_millis(&(Instant::now() - start));
                match msg {
                    Message::MidiNoteOn(chn, pitch, vel) => {
                        println!("{} midi-note-on {} {} {}",
                                 t,
                                 chn,
                                 pitch,
                                 vel);
                    }
                    Message::MidiNoteOff(chn, pitch) => {
                        println!("{} midi-note-off {} {}", t, chn, pitch);
                    }
                    Message::MidiCtl(chn, ctl, val) => {
                        println!("{} midi-ctrl {} {} {}", t, chn, ctl, val);
                    }
                    _ => (),
                }
            }
        });
        Debug {}
    }
}

impl Backend for Debug {
    fn drain(&mut self) {}
}
