pub struct Event {
    pub value: f64,
    pub duration: f64,
}

pub struct MidiState {
    pub event: Event,
}

impl MidiState {
    pub fn new() -> MidiState {
        MidiState {
            event: Event {
                value: 0.0,
                duration: 0.0,
            },
        }
    }
}
