/// A segment of time, analogous to a "bar" in musical notation
#[derive(Copy, Clone, Debug)]
pub struct Cycle {
    /// Duration in milliseconds
    pub dur: f32,
    /// Current revision of the cycle
    pub rev: usize,
}

impl Cycle {
    pub fn new() -> Cycle {
        Cycle { dur: 0f32, rev: 0 }
    }
}

#[derive(Copy, Clone, Debug)]
pub struct Event {
    pub track: u32,
    pub onset: f32,
    pub dur: f32,
    pub value: f32,
}

impl Event {
    pub fn new(track: u32, onset: f32, dur: f32, value: f32) -> Event {
        Event {
            track: track,
            onset: onset,
            dur: dur,
            value: value,
        }
    }
}

#[derive(Clone, Debug)]
pub struct SeqState {
    pub cycle: Cycle,
    pub events: Vec<Event>,
}

impl SeqState {
    pub fn new() -> SeqState {
        SeqState {
            cycle: Cycle::new(),
            events: Vec::new(),
        }
    }
}
