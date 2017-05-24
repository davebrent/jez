use unit::Event;


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
