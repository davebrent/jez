mod seq;
mod words;

use std::collections::HashMap;
use std::convert::From;
use std::sync::mpsc::{Receiver, Sender};
use std::time::Duration;

use err::RuntimeErr;
use interp::{InterpState, InterpResult, Interpreter};
use lang::Instr;
use math::millis_to_dur;
use unit::{Event, Message, Unit};

use self::seq::SeqState;
use self::words::{binlist, cycle, degrade, every, graycode, hopjump, linear,
                  palindrome, repeat, rev, reverse, rotate, shuffle, simul,
                  track};


type SpuKeyword = fn(&mut SeqState, &mut InterpState) -> InterpResult;

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
            interp: &mut Interpreter<SeqState>)
            -> Result<(), RuntimeErr> {
        self.events.clear();
        self.cycle += 1;

        interp.data.cycle.rev = self.cycle;
        interp.data.tracks.clear();
        interp.state.reset();

        match interp.eval(instrs) {
            Err(err) => Err(err),
            Ok(_) => {
                let res = interp
                    .data
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
    id: &'static str,
    interp: Interpreter<SeqState>,
    channel: Sender<Message>,
    input_channel: Receiver<Message>,
    instrs: Vec<Instr>,
    tracks: Vec<Track>,
}

impl Spu {
    /// Returns a new SPU if there are instructions to execute
    pub fn new(id: &'static str,
               instrs: Option<&[Instr]>,
               channel: Sender<Message>,
               input_channel: Receiver<Message>)
               -> Option<Self> {

        match instrs {
            None => None,
            Some(instrs) => {
                let mut words: HashMap<&'static str,
                                       SpuKeyword> = HashMap::new();
                words.insert("binlist", binlist);
                words.insert("cycle", cycle);
                words.insert("degrade", degrade);
                words.insert("every", every);
                words.insert("graycode", graycode);
                words.insert("hopjump", hopjump);
                words.insert("linear", linear);
                words.insert("palindrome", palindrome);
                words.insert("repeat", repeat);
                words.insert("rev", rev);
                words.insert("reverse", reverse);
                words.insert("rotate", rotate);
                words.insert("shuffle", shuffle);
                words.insert("simul", simul);
                words.insert("track", track);

                let mut interp = Interpreter::new(words, SeqState::new());
                match interp.eval(instrs) {
                    Err(err) => {
                        let msg = Message::Error(id, From::from(err));
                        channel.send(msg).unwrap();
                        None
                    }
                    Ok(_) => {
                        let mut tracks = Vec::new();
                        for t in &mut interp.data.tracks {
                            let mut track = Track::new(t.num, channel.clone());
                            track.set(t.dur, t.events.clone());
                            tracks.push(track);
                        }

                        Some(Spu {
                                 id: id,
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
                match track.eval(&self.instrs, &mut self.interp) {
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
