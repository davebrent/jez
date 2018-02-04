use std::collections::HashMap;
use std::sync::mpsc::Sender;
use std::time::Duration;

use lang::hash_str;

use super::filters::Filter;
use super::math::{Curve, dur_to_millis, millis_to_dur, path_to_curve,
                  point_on_curve};
use super::msgs::{Command, Destination, Event, EventValue};

#[derive(Debug)]
pub struct MidiProcessor {
    output: Sender<Command>,
    off_events: Vec<(Duration, u8, u8)>,
    ctl_events: Vec<(Duration, f64, u8, u8, Curve)>,
    last_update: Duration,
}

impl MidiProcessor {
    pub fn new(output: Sender<Command>) -> Self {
        MidiProcessor {
            output: output,
            off_events: Vec::new(),
            ctl_events: Vec::new(),
            last_update: Duration::new(0, 0),
        }
    }

    pub fn update(&mut self, elapsed: &Duration) {
        let delta = match elapsed.checked_sub(self.last_update) {
            Some(dur) => dur,
            None => Duration::new(0, 0),
        };
        self.last_update = *elapsed;
        self.update_ctl_events(&delta);
        self.update_off_events(&delta);
    }

    pub fn stop(&mut self) {
        while let Some((_, chan, pitch)) = self.off_events.pop() {
            self.output.send(Command::MidiNoteOff(chan, pitch)).ok();
        }
    }

    pub fn process(&mut self, event: Event) {
        match event.value {
            EventValue::Trigger(val) => self.handle_trg_event(event, val as u8),
            EventValue::Curve(curve) => self.handle_ctl_event(event, curve),
        };
    }

    fn handle_trg_event(&mut self, event: Event, ptch: u8) {
        let (chan, vel) = match event.dest {
            Destination::Midi(chan, vel) => (chan, vel),
        };

        let len = self.off_events.len();
        self.off_events.retain(
            |&evt| !(evt.1 == chan && evt.2 == ptch),
        );
        if len != self.off_events.len() {
            self.output.send(Command::MidiNoteOff(chan, ptch)).ok();
        }

        self.off_events.push((millis_to_dur(event.dur), chan, ptch));
        self.off_events.sort_by(
            |a, b| b.0.partial_cmp(&a.0).unwrap(),
        );
        self.output.send(Command::MidiNoteOn(chan, ptch, vel)).ok();
    }

    fn handle_ctl_event(&mut self, event: Event, curve: Curve) {
        let (chan, ctl) = match event.dest {
            Destination::Midi(chan, vel) => (chan, vel),
        };

        let dur = millis_to_dur(event.dur);
        let msg = Command::MidiCtl(chan, ctl, curve[0] as u8);

        self.ctl_events.push((dur, 0.0, chan, ctl, curve));
        self.ctl_events.sort_by(
            |a, b| b.0.partial_cmp(&a.0).unwrap(),
        );
        self.output.send(msg).ok();
    }

    fn update_ctl_events(&mut self, delta: &Duration) {
        for evt in &mut self.ctl_events {
            evt.1 += dur_to_millis(delta) / dur_to_millis(&evt.0);
        }

        for evt in &mut self.ctl_events {
            let t = evt.1;
            let val = point_on_curve(t, &evt.4);
            let msg = Command::MidiCtl(evt.2, evt.3, val[1] as u8);
            self.output.send(msg).ok();
        }

        self.ctl_events.retain(|&evt| evt.1 < 1.0);
    }

    fn update_off_events(&mut self, delta: &Duration) {
        let zero = Duration::new(0, 0);

        for evt in &mut self.off_events {
            evt.0 = match evt.0.checked_sub(*delta) {
                Some(dur) => dur,
                None => zero,
            }
        }

        while let Some((dur, chan, pitch)) = self.off_events.pop() {
            if dur != zero {
                self.off_events.push((dur, chan, pitch));
                break;
            } else {
                let msg = Command::MidiNoteOff(chan, pitch);
                self.output.send(msg).ok();
            }
        }
    }
}

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

impl Filter for MidiVelocityMapper {
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
