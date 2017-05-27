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

use unit::{Keyword, eval, Event, EventValue, Message, Interpreter,
           InterpState, add, subtract, multiply, divide, print, RuntimeErr,
           InterpResult};
use lang::{hash_str, Instr};
use math::{Curve, point_on_curve, dur_to_millis, millis_to_dur};

use self::state::{MidiState, MidiMessage};
use self::words::{event_value, event_duration, event_track, ctrlout, noteout};


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
        mpu_words.insert(hash_str("event_track"), event_track);
        mpu_words.insert(hash_str("noteout"), noteout);
        mpu_words.insert(hash_str("ctrlout"), ctrlout);

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
    instrs_out_note: Vec<Instr>,
    instrs_out_ctrl: Vec<Instr>,
    off_events: Vec<(Duration, u8, u8)>,
    ctl_events: Vec<(Duration, f64, u8, u8, Curve)>,
}

impl Mpu {
    pub fn new(id: u8,
               instrs_out_note: Option<&[Instr]>,
               instrs_out_ctrl: Option<&[Instr]>,
               channel: Sender<Message>,
               input_channel: Receiver<Message>)
               -> Option<Self> {
        if instrs_out_note.is_none() && instrs_out_ctrl.is_none() {
            return None;
        }

        let instrs_out_note = match instrs_out_note {
            Some(instrs) => instrs.to_vec(),
            None => Vec::new(),
        };

        let instrs_out_ctrl = match instrs_out_ctrl {
            Some(instrs) => instrs.to_vec(),
            None => Vec::new(),
        };

        Some(Mpu {
                 id: id,
                 interp_state: InterpState::new(),
                 interp: MpuInterp::new(),
                 channel: channel,
                 input_channel: input_channel,
                 instrs_out_note: instrs_out_note,
                 instrs_out_ctrl: instrs_out_ctrl,
                 off_events: Vec::new(),
                 ctl_events: Vec::new(),
             })
    }

    fn handle_trg_event(&mut self, event: Event) {
        let instrs = self.instrs_out_note.as_slice();

        self.interp_state.reset();
        self.interp.midi_state.event = event;

        match eval(instrs, &mut self.interp_state, &mut self.interp) {
            Err(err) => {
                self.channel
                    .send(Message::HasError(self.id, err))
                    .unwrap();
            }
            Ok(_) => {
                match self.interp.midi_state.message {
                    MidiMessage::None => return,
                    MidiMessage::Note {
                        channel: chan,
                        pitch: ptch,
                        velocity: vel,
                        duration: dur,
                    } => {
                        self.off_events.push((millis_to_dur(dur), chan, ptch));
                        self.off_events
                            .sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap());
                        self.channel
                            .send(Message::MidiNoteOn(chan, ptch, vel))
                            .unwrap();
                    }
                    _ => {
                        let err = RuntimeErr::WrongType;
                        self.channel
                            .send(Message::HasError(self.id, err))
                            .unwrap();
                    }
                }
            }
        }
    }

    fn handle_ctl_event(&mut self, event: Event, curve: Curve) {
        let instrs = self.instrs_out_ctrl.as_slice();

        self.interp_state.reset();
        self.interp.midi_state.event = event;

        match eval(instrs, &mut self.interp_state, &mut self.interp) {
            Err(err) => {
                self.channel
                    .send(Message::HasError(self.id, err))
                    .unwrap();
            }
            Ok(_) => {
                match self.interp.midi_state.message {
                    MidiMessage::None => return,
                    MidiMessage::Ctrl {
                        channel: chan,
                        ctrl: ctl,
                    } => {
                        let dur = millis_to_dur(event.dur);
                        let msg = Message::MidiCtl(chan, ctl, curve[0] as u8);
                        self.ctl_events.push((dur, 0.0, chan, ctl, curve));
                        self.ctl_events
                            .sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap());
                        self.channel.send(msg).unwrap();
                    }
                    _ => {
                        let err = RuntimeErr::WrongType;
                        self.channel
                            .send(Message::HasError(self.id, err))
                            .unwrap();
                    }
                }
            }
        }
    }

    fn process_ctl_events(&mut self, delta: &Duration) {
        for evt in &mut self.ctl_events {
            evt.1 += dur_to_millis(delta) / dur_to_millis(&evt.0);
        }

        for evt in &mut self.ctl_events {
            let t = evt.1;
            let val = point_on_curve(t, &evt.4);
            let msg = Message::MidiCtl(evt.2, evt.3, val[1] as u8);
            self.channel.send(msg).unwrap();
        }

        self.ctl_events.retain(|&evt| evt.1 < 1.0);
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
                    Message::SeqEvent(event) => {
                        match event.value {
                            EventValue::Trigger(_) => {
                                self.handle_trg_event(event)
                            }
                            EventValue::Curve(curve) => {
                                self.handle_ctl_event(event, curve)
                            }
                        }
                    }
                    _ => (),
                }
            }

            thread::sleep(res);
        }
    }
}
