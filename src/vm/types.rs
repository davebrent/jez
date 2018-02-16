use rand::{SeedableRng, StdRng};
use std::rc::Rc;

use super::interp::{InterpResult, InterpState};
use super::math::Curve;

#[derive(Copy, Clone, Debug, PartialEq, Serialize)]
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
}

impl Track {
    pub fn new(id: usize, func: u64) -> Track {
        Track {
            id: id,
            func: func,
            effects: Vec::new(),
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
