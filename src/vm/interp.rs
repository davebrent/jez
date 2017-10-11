use std::rc::Rc;

use interp::{InterpResult, InterpState};

use rand::{SeedableRng, StdRng};

use super::audio::AudioContext;
use super::filters::Filter;
use super::msgs::Event;

pub type ExtKeyword = fn(&mut ExtState, &mut InterpState) -> InterpResult;

#[derive(Clone)]
pub struct Track {
    pub id: usize,
    pub func: u64,
    pub filters: Vec<Rc<Filter>>,
}

impl Track {
    pub fn new(id: usize, func: u64) -> Track {
        Track {
            id: id,
            func: func,
            filters: Vec::new(),
        }
    }
}

#[derive(Clone)]
pub struct ExtState {
    pub revision: usize,
    pub events: Vec<Event>,
    pub tracks: Vec<Track>,
    pub duration: f64,
    pub audio: AudioContext,
    pub rng: StdRng,
}

impl ExtState {
    pub fn new() -> ExtState {
        ExtState {
            revision: 0,
            events: Vec::new(),
            tracks: Vec::new(),
            duration: 0.0,
            audio: AudioContext::new(),
            rng: StdRng::from_seed(&[0, 0, 0, 0]),
        }
    }
}
