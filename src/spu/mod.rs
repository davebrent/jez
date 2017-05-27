//! # Sequence Processing Unit
//!
//! A sequence & pattern generator. Functions are Heavily influenced by
//! [Tidal Cycles][1] and the writing of [Godfrey Toussaint][2].
//!
//!   [1]: https://tidalcycles.org/
//!   [2]: https://link.springer.com/chapter/10.1007/11589440_20
//!
//! At the start of each cycle the sequencers instructions are evaluated. The
//! output of which is a series of events to be written to the units output
//! channel at a specified time relative to the start of the cycle.
//!
//! Events are time tagged by recursively subdividing the top most list against
//! a total cycle length (ms). Lists may be nested to generate more interesting
//! results.

mod seq;
mod words;

use std::collections::HashMap;
use std::sync::Mutex;
use std::sync::mpsc::{Sender, Receiver};
use std::time::{Duration, Instant};
use std::thread;

use unit::{Keyword, Message, Interpreter, InterpState, eval, add, subtract,
           multiply, divide, print, RuntimeErr, InterpResult, Event};
use lang::{hash_str, Instr};
use math::millis_to_dur;

use self::seq::SeqState;
use self::words::{repeat, every, reverse, shuffle, rotate, degrade, cycle,
                  palindrome, hopjump, track, linear};


type SpuKeyword = fn(&mut SeqState, &mut InterpState) -> InterpResult;

/// Interpreter for `spu` keywords
pub struct SpuInterp {
    built_ins: HashMap<u64, Keyword>,
    spu_words: HashMap<u64, SpuKeyword>,
    seq_state: SeqState,
}

impl SpuInterp {
    pub fn new() -> SpuInterp {
        let mut built_ins: HashMap<u64, Keyword> = HashMap::new();
        built_ins.insert(hash_str("add"), add);
        built_ins.insert(hash_str("subtract"), subtract);
        built_ins.insert(hash_str("multiply"), multiply);
        built_ins.insert(hash_str("divide"), divide);
        built_ins.insert(hash_str("print"), print);

        let mut spu_words: HashMap<u64, SpuKeyword> = HashMap::new();
        spu_words.insert(hash_str("repeat"), repeat);
        spu_words.insert(hash_str("every"), every);
        spu_words.insert(hash_str("reverse"), reverse);
        spu_words.insert(hash_str("shuffle"), shuffle);
        spu_words.insert(hash_str("rotate"), rotate);
        spu_words.insert(hash_str("degrade"), degrade);
        spu_words.insert(hash_str("cycle"), cycle);
        spu_words.insert(hash_str("palindrome"), palindrome);
        spu_words.insert(hash_str("hopjump"), hopjump);
        spu_words.insert(hash_str("track"), track);
        spu_words.insert(hash_str("linear"), linear);

        SpuInterp {
            built_ins: built_ins,
            spu_words: spu_words,
            seq_state: SeqState::new(),
        }
    }
}

impl Interpreter for SpuInterp {
    fn eval(&mut self, word: u64, state: &mut InterpState) -> InterpResult {
        match self.built_ins.get(&word) {
            Some(func) => func(state),
            None => {
                match self.spu_words.get(&word) {
                    None => Err(RuntimeErr::UnknownKeyword(word)),
                    Some(func) => func(&mut self.seq_state, state),
                }
            }
        }

    }
}

/// Sequencer processing unit
pub struct Spu {
    id: u8,
    interp_state: InterpState,
    interp: SpuInterp,
    channel: Sender<Message>,
    input_channel: Receiver<Message>,
    instrs: Mutex<Vec<Instr>>,
}

impl Spu {
    /// Returns a new SPU if there are instructions to execute
    pub fn new(id: u8,
               instrs: Option<&[Instr]>,
               channel: Sender<Message>,
               input_channel: Receiver<Message>)
               -> Option<Self> {
        match instrs {
            None => None,
            Some(instrs) => {
                Some(Spu {
                         id: id,
                         interp_state: InterpState::new(),
                         interp: SpuInterp::new(),
                         channel: channel.clone(),
                         input_channel: input_channel,
                         instrs: Mutex::new(instrs.to_vec()),
                     })
            }
        }
    }

    /// Returns a series of events for the current cycle
    fn get_events(&mut self) -> Option<Vec<Event>> {
        let instrs = self.instrs.lock().unwrap();
        let instrs = instrs.as_slice();

        self.interp.seq_state.events.drain(..);
        self.interp_state.reset();

        match eval(instrs, &mut self.interp_state, &mut self.interp) {
            Err(err) => {
                let msg = Message::HasError(self.id, err);
                self.channel.send(msg).unwrap();
                None
            }
            Ok(_) => {
                self.interp.seq_state.cycle.rev += 1;
                Some(self.interp.seq_state.events.clone())
            }
        }
    }

    /// Returns true if the unit should stop processing
    ///
    /// Signal is generated by sending a stop message to units input channel
    fn should_stop(&self) -> bool {
        match self.input_channel.try_recv() {
            Ok(msg) => {
                match msg {
                    Message::Stop => true,
                    _ => false,
                }
            }
            _ => false,
        }
    }

    /// Run sequencer forever blocking until 'callback' returns no more
    pub fn run_forever(&mut self) {
        let res = Duration::new(0, 1000000); // 1ms

        while let Some(mut events) = self.get_events() {
            if self.should_stop() {
                break;
            }

            // Sort events by time in descending order
            events.sort_by(|a, b| b.onset.partial_cmp(&a.onset).unwrap());

            let start = Instant::now();
            let end = millis_to_dur(self.interp.seq_state.cycle.dur);

            while let Some(event) = events.pop() {
                if start.elapsed() < millis_to_dur(event.onset) {
                    events.push(event);
                    thread::sleep(res);
                    continue;
                }
                self.channel.send(Message::SeqEvent(event)).unwrap();
            }

            while start.elapsed() < end {
                thread::sleep(res);
            }
        }
    }
}
