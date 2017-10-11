use interp::{InterpResult, InterpState};

use rand::{SeedableRng, StdRng};

use super::audio::AudioContext;
use super::msgs::Event;

pub type ExtKeyword = fn(&mut ExtState, &mut InterpState) -> InterpResult;

pub struct ExtState {
    pub revision: usize,
    pub events: Vec<Event>,
    pub tracks: Vec<(usize, u64)>,
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
