use rand::{SeedableRng, StdRng};
use std::rc::Rc;

use super::interp::{InterpResult, InterpState};
use super::math::Curve;
use super::time::Priority;

#[derive(Copy, Clone, Debug, PartialEq, Serialize, Eq)]
pub enum Destination {
    Midi(u8, u8),
}

#[derive(Copy, Clone, Debug, PartialEq, Serialize)]
pub enum EventValue {
    Trigger(f64),
    Curve(Curve),
}

#[derive(Copy, Clone, Debug, PartialEq, Serialize)]
pub struct Event {
    pub dest: Destination,
    pub onset: f64,
    pub dur: f64,
    pub value: EventValue,
}

#[derive(Copy, Clone, Debug, PartialEq, Serialize)]
pub enum Command {
    Event(Event),
    MidiCtl(u8, u8, u8),
    MidiNoteOff(u8, u8),
    MidiNoteOn(u8, u8, u8),
    Stop,
    Reload,
    Clock,
    Track(usize, usize, u64),
}

impl Priority for Command {
    fn priority(&self) -> usize {
        match *self {
            Command::MidiNoteOff(_, _) => 0,
            Command::Stop => 1,
            Command::Reload => 2,
            Command::Clock => 3,
            Command::Track(_, _, _) => 4,
            Command::Event(_) => 5,
            Command::MidiNoteOn(_, _, _) => 6,
            Command::MidiCtl(_, _, _) => 7,
        }
    }
}

pub trait Effect {
    fn apply(&mut self, dur: f64, events: &[Event]) -> Vec<Event>;
}

pub type Result = InterpResult;
pub type Keyword = fn(&mut SeqState, &mut InterpState) -> InterpResult;

#[derive(Clone)]
pub struct Track {
    pub id: usize,
    pub func: u64,
    pub effects: Vec<Rc<Effect>>,
    pub real_time: f64,
    pub schedule_time: f64,
}

impl Track {
    pub fn new(id: usize, func: u64) -> Track {
        Track {
            id: id,
            func: func,
            effects: Vec::new(),
            real_time: 0.0,
            schedule_time: 0.0,
        }
    }
}

#[derive(Clone)]
pub struct SeqState {
    pub revision: usize,
    pub events: Vec<Event>,
    pub tracks: Vec<Track>,
    pub duration: f64,
    pub rng: StdRng,
}

impl SeqState {
    pub fn new() -> SeqState {
        SeqState {
            revision: 0,
            events: Vec::new(),
            tracks: Vec::new(),
            duration: 0.0,
            rng: StdRng::from_seed(&[0, 0, 0, 0]),
        }
    }

    pub fn reset(&mut self, rev: usize) {
        self.revision = rev;
        self.duration = 0.0;
        self.events.clear();
    }
}
