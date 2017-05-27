//! # MIDI Processing Unit
//!
//! The MPU reads events from its input channel, evaluating its instructions
//! against each event. Then dispatching any generated MIDI events.

mod words;
mod state;

use std::collections::HashMap;
use std::sync::mpsc::{Sender, Receiver};
use std::time::{Duration, Instant};
use std::thread;

use unit::{Curve, Keyword, eval, Event, EventValue, Message, Interpreter,
           InterpState, add, subtract, multiply, divide, print, RuntimeErr,
           InterpResult};
use lang::{hash_str, Instr};

use self::state::MidiState;
use self::words::{event_value, event_duration, makenote, noteout};


type MpuKeyword = fn(&mut MidiState, &mut InterpState) -> InterpResult;

pub struct MpuInterp {
    built_ins: HashMap<u64, Keyword>,
    mpu_words: HashMap<u64, MpuKeyword>,
    midi_state: MidiState,
}

impl MpuInterp {
    pub fn new() -> MpuInterp {
        let mut built_ins: HashMap<u64, Keyword> = HashMap::new();
        built_ins.insert(hash_str("add"), add);
        built_ins.insert(hash_str("subtract"), subtract);
        built_ins.insert(hash_str("multiply"), multiply);
        built_ins.insert(hash_str("divide"), divide);
        built_ins.insert(hash_str("print"), print);

        let mut mpu_words: HashMap<u64, MpuKeyword> = HashMap::new();
        mpu_words.insert(hash_str("event_value"), event_value);
        mpu_words.insert(hash_str("event_duration"), event_duration);
        mpu_words.insert(hash_str("makenote"), makenote);
        mpu_words.insert(hash_str("noteout"), noteout);

        MpuInterp {
            built_ins: built_ins,
            mpu_words: mpu_words,
            midi_state: MidiState::new(),
        }
    }
}

impl Interpreter for MpuInterp {
    fn eval(&mut self, word: u64, state: &mut InterpState) -> InterpResult {
        match self.built_ins.get(&word) {
            Some(func) => func(state),
            None => {
                match self.mpu_words.get(&word) {
                    None => Err(RuntimeErr::UnknownKeyword(word)),
                    Some(func) => func(&mut self.midi_state, state),
                }
            }
        }
    }
}

pub struct Mpu {
    id: u8,
    interp_state: InterpState,
    interp: MpuInterp,
    channel: Sender<Message>,
    input_channel: Receiver<Message>,
    instrs: Vec<Instr>,
    off_events: Vec<(Duration, u8, u8)>,
    ctl_events: Vec<(Duration, f64, Curve)>,
}

fn from_millis(millis: f32) -> Duration {
    let secs = (millis / 1000f32).floor();
    let nanos = (millis - (secs * 1000f32)) * 1000000f32;
    Duration::new(secs as u64, nanos as u32)
}

fn to_millis(dur: &Duration) -> f64 {
    let secs = dur.as_secs() as f64 * 1000.0;
    let nanos = dur.subsec_nanos() as f64 / 1000.0;
    secs + nanos
}

/// Get a point 't' on a cubic bezier curve
fn get_point(t: f64, curve: Curve) -> [f32; 2] {
    let t = t as f32;
    let p0x = curve[0];
    let p0y = curve[1];
    let p1x = curve[2];
    let p1y = curve[3];
    let p2x = curve[4];
    let p2y = curve[5];
    let x = (1.0 - t) * (1.0 - t) * p0x + 2.0 * (1.0 - t) * t * p1x +
            t * t * p2x;
    let y = (1.0 - t) * (1.0 - t) * p0y + 2.0 * (1.0 - t) * t * p1y +
            t * t * p2y;
    [x, y]
}

impl Mpu {
    pub fn new(id: u8,
               instrs: Option<&[Instr]>,
               channel: Sender<Message>,
               input_channel: Receiver<Message>)
               -> Option<Self> {
        match instrs {
            None => None,
            Some(instrs) => {
                Some(Mpu {
                         id: id,
                         interp_state: InterpState::new(),
                         interp: MpuInterp::new(),
                         channel: channel,
                         input_channel: input_channel,
                         instrs: instrs.to_vec(),
                         off_events: Vec::new(),
                         ctl_events: Vec::new(),
                     })
            }
        }
    }

    fn handle_seq_event(&mut self, event: Event) {
        let instrs = self.instrs.as_slice();
        match eval(instrs, &mut self.interp_state, &mut self.interp) {
            Err(err) => {
                let msg = Message::HasError(self.id, err);
                self.channel.send(msg).unwrap();
            }
            Ok(_) => {
                match event.value {
                    EventValue::Trigger(val) => {
                        let val = val as u8;
                        self.off_events.push((from_millis(event.dur), 0, val));
                        self.off_events
                            .sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap());
                        self.channel
                            .send(Message::MidiNoteOn(0, val, 127))
                            .unwrap();
                    }
                    EventValue::Curve(curve) => {
                        let dur = from_millis(event.dur);
                        self.ctl_events.push((dur, 0.0, curve));
                        self.ctl_events
                            .sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap());
                        self.channel
                            .send(Message::MidiCtl(0, 0, curve[0] as u8))
                            .unwrap();
                    }
                }
            }
        }
    }

    fn process_ctl_events(&mut self, delta: &Duration) {
        // Advance 't' for each curve
        for evt in &mut self.ctl_events {
            evt.1 += to_millis(&evt.0) / to_millis(delta);
        }

        for evt in &mut self.ctl_events {
            let t = evt.1;
            let val = get_point(t, evt.2);
            let msg = Message::MidiCtl(0, 0, val[1] as u8);
            self.channel.send(msg).unwrap();
        }

        // Remove any event with a t >= 1
    }

    fn process_off_events(&mut self, delta: &Duration) {
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
                let msg = Message::MidiNoteOff(chan, pitch);
                self.channel.send(msg).unwrap();
            }
        }
    }

    pub fn run_forever(&mut self) {
        let res = Duration::new(0, 1000000); // 1ms
        let mut previous = Instant::now();

        loop {
            let now = Instant::now();
            let delta = now.duration_since(previous);
            previous = now;

            self.process_ctl_events(&delta);
            self.process_off_events(&delta);

            if let Ok(msg) = self.input_channel.try_recv() {
                match msg {
                    Message::Stop => break,
                    Message::SeqEvent(event) => self.handle_seq_event(event),
                    _ => (),
                }
            }

            thread::sleep(res);
        }
    }
}
