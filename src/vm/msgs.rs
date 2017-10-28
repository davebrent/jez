use super::math::Curve;

#[derive(Copy, Clone, Debug, PartialEq, Serialize)]
pub enum Destination {
    Midi(u8, u8),
    Synth(u64, u64),
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
    AudioSettings(usize, usize, usize),
    Event(Event),
    MidiCtl(u8, u8, u8),
    MidiNoteOff(u8, u8),
    MidiNoteOn(u8, u8, u8),
    Stop,
    Reload,
}
