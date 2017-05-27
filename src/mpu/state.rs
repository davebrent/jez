use unit::{Event, EventValue};

pub enum MidiMessage {
    None,
    Ctrl { channel: u8, ctrl: u8 },
    Note {
        channel: u8,
        pitch: u8,
        velocity: u8,
        duration: f64,
    },
}

pub struct MidiState {
    pub event: Event,
    pub message: MidiMessage,
}

impl MidiState {
    pub fn new() -> MidiState {
        MidiState {
            message: MidiMessage::None,
            event: Event {
                track: 0,
                onset: 0.0,
                dur: 0.0,
                value: EventValue::Trigger(0.0),
            },
        }
    }
}
