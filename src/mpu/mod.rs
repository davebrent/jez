//! # MIDI Processing Unit
//!
//! The MPU reads events from its input channel, evaluating its instructions
//! against each event. Then dispatching any generated MIDI events.

mod words;
mod state;

use std::collections::HashMap;
use std::sync::mpsc::{Sender, Receiver};

/*
use jack_sys as j;
use jack::prelude::{AsyncClient, Client, JackControl, MidiInPort, MidiInSpec,
                    Port, MidiOutPort, MidiOutSpec, ClosureProcessHandler,
                    ProcessScope, RawMidi, client_options};
*/

use unit::{Keyword, eval, Message, Interpreter, InterpState, add, subtract,
           multiply, divide, print, RuntimeErr, InterpResult};
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
    //jack_client: Client,
    //out_port: Port<MidiOutSpec>,
}

impl Mpu {
    pub fn new(id: u8,
               instrs: Option<&[Instr]>,
               channel: Sender<Message>,
               input_channel: Receiver<Message>)
               -> Option<Mpu> {
        //let opts = client_options::NO_START_SERVER;
        //let (client, status) = Client::new("han-mpu", opts).unwrap();
        //
        //let port = client
        //    .register_port("midiout_1", MidiOutSpec::default())
        //    .unwrap();

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
                        //jack_client: client,
                        //out_port: port,
                     })
            }
        }
    }

    fn handle_trigger_event(&mut self, val: f32, dur: f32) {
        let instrs = self.instrs.as_slice();
        let res = eval(instrs, &mut self.interp_state, &mut self.interp);
        match res {
            Err(err) => {
                let msg = Message::HasError(self.id, err);
                self.channel.send(msg).unwrap();
            }
            Ok(_) => {
                println!("MidiOut {} {}", val, dur);
            }
        }
    }

    pub fn run_forever(&mut self) {
        while let Ok(msg) = self.input_channel.recv() {
            match msg {
                Message::TriggerEvent(val, dur) => {
                    self.handle_trigger_event(val, dur);
                }
                _ => (),
            }
        }
    }
}
