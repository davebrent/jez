use std::thread;
use std::time::{Duration, Instant};

use err::JezErr;
use math::Curve;


#[derive(Copy, Clone, Debug, PartialEq, Serialize)]
pub enum EventValue {
    Trigger(f64),
    Curve(Curve),
}

#[derive(Copy, Clone, Debug, PartialEq, Serialize)]
pub struct Event {
    pub track: u32,
    pub onset: f64,
    pub dur: f64,
    pub value: EventValue,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize)]
pub enum Message {
    Error(u8, JezErr),
    MidiCtl(u8, u8, u8),
    MidiNoteOff(u8, u8),
    MidiNoteOn(u8, u8, u8),
    SeqEvent(Event),
    Stop,
    Reload,
}

pub trait Unit {
    fn tick(&mut self, delta: &Duration) -> bool;

    fn run_forever(&mut self, res: Duration) {
        let mut previous = Instant::now();
        loop {
            let now = Instant::now();
            let delta = now.duration_since(previous);
            if self.tick(&delta) {
                return;
            }
            previous = now;
            thread::sleep(res);
        }
    }
}
