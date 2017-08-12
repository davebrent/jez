use interp::{InterpState, InterpResult};
use super::msgs::Event;

pub type ExtKeyword = fn(&mut ExtState, &mut InterpState) -> InterpResult;

#[derive(Clone, Debug)]
pub struct ExtState {
    pub revision: usize,
    pub events: Vec<Event>,
    pub tracks: Vec<(usize, u64)>,
    pub duration: f64,
}

impl ExtState {
    pub fn new() -> ExtState {
        ExtState {
            revision: 0,
            events: Vec::new(),
            tracks: Vec::new(),
            duration: 0.0,
        }
    }
}
