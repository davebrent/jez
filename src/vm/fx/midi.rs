use std::collections::HashMap;

use lang::hash_str;
use vm::math::path_to_curve;
use vm::types::{Destination, Effect, Event, EventValue};

type MidiMap = HashMap<u64, u8>;

fn volca_fm_map() -> MidiMap {
    let mut map: MidiMap = HashMap::new();
    map.insert(hash_str("octave"), 40);
    map.insert(hash_str("velocity"), 41);
    map.insert(hash_str("modulator_attack"), 42);
    map.insert(hash_str("modulator_decay"), 43);
    map.insert(hash_str("carrier_attack"), 44);
    map.insert(hash_str("carrier_decay"), 45);
    map.insert(hash_str("lfo_rate"), 46);
    map.insert(hash_str("lfo_pitch_depth"), 47);
    map.insert(hash_str("algorithm"), 48);
    map
}

fn volca_sample_map() -> MidiMap {
    let mut map: MidiMap = HashMap::new();
    map.insert(hash_str("level"), 7);
    // XXX: Not a real parameter but just to make life easier
    map.insert(hash_str("velocity"), 7);
    map.insert(hash_str("pan"), 10);
    map.insert(hash_str("sample_start_point"), 40);
    map.insert(hash_str("sample_length"), 41);
    map.insert(hash_str("hi_cut"), 42);
    map.insert(hash_str("speed"), 43);
    map.insert(hash_str("pitch_eg_int"), 44);
    map.insert(hash_str("pitch_eg_attack"), 45);
    map.insert(hash_str("pitch_eg_decay"), 46);
    map.insert(hash_str("amp_eg_attack"), 47);
    map.insert(hash_str("amp_eg_decay"), 48);
    map
}

fn device_map() -> HashMap<u64, MidiMap> {
    let mut map = HashMap::new();
    map.insert(hash_str("volca_fm"), volca_fm_map());
    map.insert(hash_str("volca_sample"), volca_sample_map());
    map
}

fn mapping(device: u64, param: u64) -> Option<u8> {
    let devices = device_map();
    let map = match devices.get(&device) {
        Some(map) => map,
        None => return None,
    };
    match map.get(&param) {
        Some(target) => Some(*target),
        None => None,
    }
}

/// Map note velocities to CC messages
#[derive(Clone, Debug)]
pub struct MidiVelocityMapper {
    ctrl: u8,
}

impl MidiVelocityMapper {
    pub fn new(device: u64, param: u64) -> Option<MidiVelocityMapper> {
        match mapping(device, param) {
            Some(ctrl) => Some(MidiVelocityMapper { ctrl: ctrl }),
            None => None,
        }
    }

    fn map(&self, event: Event) -> Option<Event> {
        let mut event = match event.value {
            EventValue::Curve(_) => return None,
            EventValue::Trigger(_) => event,
        };

        match event.dest {
            Destination::Midi(channel, velocity) => {
                event.dest = Destination::Midi(channel, self.ctrl);
                event.value = EventValue::Curve(path_to_curve(
                    &[event.onset, f64::from(velocity)],
                    &[event.dur, f64::from(velocity)],
                ));
                Some(event)
            }
        }
    }
}

impl Effect for MidiVelocityMapper {
    fn apply(&mut self, _: f64, events: &[Event]) -> Vec<Event> {
        let mut output = Vec::with_capacity(events.len());
        for event in events {
            let event = *event;
            if let Some(cc) = self.map(event) {
                output.push(cc)
            }
            output.push(event);
        }
        output
    }
}
