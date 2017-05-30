use unit::Event;


/// A segment of time, analogous to a "bar" in musical notation
#[derive(Copy, Clone, Debug)]
pub struct Cycle {
    /// Duration in milliseconds
    pub dur: f64,
    /// Current revision of the cycle
    pub rev: usize,
}

impl Cycle {
    pub fn new() -> Cycle {
        Cycle { dur: 0.0, rev: 0 }
    }
}

#[derive(Clone, Debug)]
pub struct SeqTrack {
    pub num: usize,
    pub dur: f64,
    pub events: Vec<Event>,
}

#[derive(Clone, Debug)]
pub struct SeqState {
    pub cycle: Cycle,
    pub tracks: Vec<SeqTrack>,
}

impl SeqState {
    pub fn new() -> SeqState {
        SeqState {
            cycle: Cycle::new(),
            tracks: Vec::new(),
        }
    }
}
