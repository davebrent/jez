pub struct Event {
    pub value: f32,
    pub duration: f32,
}

pub struct MidiState {
    pub event: Event,
}

impl MidiState {
    pub fn new() -> MidiState {
        MidiState {
            event: Event {
                value: 0f32,
                duration: 0f32,
            },
        }
    }
}
