mod seq;
mod words;

use std::collections::HashMap;
use std::convert::From;
use std::sync::mpsc::{Receiver, Sender};
use std::time::Duration;

use err::RuntimeErr;
use lang::{hash_str, Instr};
use math::millis_to_dur;
use unit::{add, divide, eval, Event, InterpState, InterpResult, Interpreter,
           Keyword, Message, multiply, print, subtract, Unit};

use self::seq::SeqState;
use self::words::{binlist, cycle, degrade, every, graycode, hopjump, linear,
                  palindrome, repeat, rev, reverse, rotate, shuffle, simul,
                  track};


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
        built_ins.insert(hash_str("divide"), divide);
        built_ins.insert(hash_str("multiply"), multiply);
        built_ins.insert(hash_str("print"), print);
        built_ins.insert(hash_str("subtract"), subtract);

        let mut spu_words: HashMap<u64, SpuKeyword> = HashMap::new();
        spu_words.insert(hash_str("binlist"), binlist);
        spu_words.insert(hash_str("cycle"), cycle);
        spu_words.insert(hash_str("degrade"), degrade);
        spu_words.insert(hash_str("every"), every);
        spu_words.insert(hash_str("graycode"), graycode);
        spu_words.insert(hash_str("hopjump"), hopjump);
        spu_words.insert(hash_str("linear"), linear);
        spu_words.insert(hash_str("palindrome"), palindrome);
        spu_words.insert(hash_str("repeat"), repeat);
        spu_words.insert(hash_str("rev"), rev);
        spu_words.insert(hash_str("reverse"), reverse);
        spu_words.insert(hash_str("rotate"), rotate);
        spu_words.insert(hash_str("shuffle"), shuffle);
        spu_words.insert(hash_str("simul"), simul);
        spu_words.insert(hash_str("track"), track);

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

struct Track {
    pub cycle: usize,
    pub num: usize,
    pub events: Vec<Event>,
    channel: Sender<Message>,
    start: Duration,
    end: Duration,
}

impl Track {
    pub fn new(num: usize, channel: Sender<Message>) -> Track {
        Track {
            cycle: 0,
            num: num,
            channel: channel,
            events: Vec::new(),
            start: Duration::new(0, 0),
            end: Duration::new(0, 0),
        }
    }

    pub fn start(&mut self) {
        self.start = Duration::new(0, 0);
    }

    pub fn set(&mut self, len: f64, events: Vec<Event>) {
        self.end = millis_to_dur(len);
        self.events = events;
        // Sort events by time in descending order
        self.events
            .sort_by(|a, b| b.onset.partial_cmp(&a.onset).unwrap());
    }

    fn eval(&mut self,
            instrs: &[Instr],
            interp: &mut SpuInterp,
            interp_state: &mut InterpState)
            -> Result<(), RuntimeErr> {
        self.events.clear();
        self.cycle += 1;

        interp.seq_state.cycle.rev = self.cycle;
        interp.seq_state.tracks.clear();
        interp_state.reset();

        match eval(instrs, interp_state, interp) {
            Err(err) => Err(err),
            Ok(_) => {
                let res = interp
                    .seq_state
                    .tracks
                    .iter_mut()
                    .find(|t| t.num == self.num);

                match res {
                    Some(t) => {
                        self.set(t.dur, t.events.clone());
                        Ok(())
                    }
                    None => Ok(()),
                }
            }
        }
    }

    pub fn finished(&self) -> bool {
        self.start >= self.end
    }

    pub fn tick(&mut self, delta: &Duration) {
        self.start += *delta;
        while let Some(event) = self.events.pop() {
            if self.start < millis_to_dur(event.onset) {
                self.events.push(event);
                break;
            }
            self.channel.send(Message::SeqEvent(event)).unwrap();
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
    instrs: Vec<Instr>,
    tracks: Vec<Track>,
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
                let mut tracks = Vec::new();
                let mut interp_state = InterpState::new();
                let mut interp = SpuInterp::new();

                match eval(instrs, &mut interp_state, &mut interp) {
                    Err(err) => {
                        let msg = Message::Error(id, From::from(err));
                        channel.send(msg).unwrap();
                        None
                    }
                    Ok(_) => {
                        for t in &mut interp.seq_state.tracks {
                            let mut track = Track::new(t.num, channel.clone());
                            track.set(t.dur, t.events.clone());
                            tracks.push(track);
                        }

                        Some(Spu {
                                 id: id,
                                 interp_state: interp_state,
                                 interp: interp,
                                 channel: channel,
                                 input_channel: input_channel,
                                 instrs: instrs.to_vec(),
                                 tracks: tracks,
                             })
                    }
                }
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
}

impl Unit for Spu {
    fn tick(&mut self, delta: &Duration) -> bool {
        for track in &mut self.tracks {
            if track.finished() {
                match track.eval(&self.instrs,
                                 &mut self.interp,
                                 &mut self.interp_state) {
                    Err(err) => {
                        let msg = Message::Error(self.id, From::from(err));
                        self.channel.send(msg).unwrap();
                        return true;
                    }
                    _ => {
                        track.start();
                    }
                }
            }
            track.tick(delta);
        }

        self.should_stop()
    }
}
