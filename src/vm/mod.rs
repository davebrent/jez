mod fx;
mod interp;
mod math;
mod midi;
mod time;
mod types;
mod words;

use std::collections::HashMap;
use std::rc::Rc;
use std::time::Duration;

use err::Error;
use lang::hash_str;

pub use self::interp::{Instr, InterpState, Value};
use self::interp::{BaseInterpreter, Interpreter, StackTraceInterpreter};
pub use self::math::{dur_to_millis, millis_to_dur};
use self::midi::MidiProcessor;
use self::time::Clock as InternalClock;
pub use self::time::TimeEvent;
pub use self::types::{Command, Destination, Event, EventValue};
use self::types::{SeqState, Track};

pub type Clock = InternalClock<Command>;

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum Status {
    Reload,
    Stop,
    Continue,
}

fn interpreter(
    instrs: &[Instr],
) -> Result<(HashMap<u64, usize>, Box<Interpreter<SeqState>>), Error> {
    let mut interp = Box::new(StackTraceInterpreter::new(Box::new(BaseInterpreter::new(
        instrs.to_vec(),
        &words::all(),
        SeqState::new(),
    ))));

    // Create tracks as defined by block 1 (the extension block)
    if let Some(val) = try!(interp.eval_block(1)) {
        let (start, end) = try!(val.as_range());
        let state = interp.state();
        for (i, ptr) in (start..end).enumerate() {
            let sym = try!(try!(state.heap_get(ptr)).as_sym());
            let data = interp.data_mut();
            data.tracks.push(Track::new(i, sym));
        }
    }

    // Create a mapping of function names to program counters
    let mut funcs = HashMap::new();
    for (pc, instr) in instrs.iter().enumerate() {
        if let Instr::Begin(word) = *instr {
            funcs.insert(word, pc + 1);
        }
    }

    // Reset interpreter and call into `main`
    interp.data_mut().reset(0);
    interp.reset();
    match funcs.get(&hash_str("main")).map(|p| *p) {
        Some(pc) => {
            try!(interp.eval(pc));
        }
        None => (),
    };

    // Reset the interpreter again for first use
    interp.data_mut().reset(0);
    interp.reset();
    Ok((funcs, interp))
}

type Schedule = Box<FnMut(TimeEvent<Command>)>;
type Out = Box<FnMut(Command)>;
type In = Box<FnMut() -> Option<Command>>;

pub struct Machine {
    interp: Box<Interpreter<SeqState>>,
    clock: Schedule,
    sink: Out,
    input: In,
    functions: HashMap<u64, usize>,
    midi: MidiProcessor,
}

impl Machine {
    pub fn new(
        sink: Out,
        mut clock: Schedule,
        bus: Out,
        input: In,
        instrs: &[Instr],
    ) -> Result<Machine, Error> {
        // Construct the interpreter and its state
        let (funcs, mut interp) = try!(self::interpreter(instrs));

        // Schedule all tracks for the first time
        for track in &interp.data_mut().tracks {
            let track = Command::Track(track.id, 0, track.func);
            let cmd = TimeEvent::Timeout(0.0, track);
            clock(cmd);
        }

        Ok(Machine {
            sink: sink,
            clock: clock,
            input: input,
            functions: funcs,
            interp: interp,
            midi: MidiProcessor::new(bus),
        })
    }

    pub fn process(&mut self, cmd: Command, delta: &Duration) -> Result<Status, Error> {
        let status = try!(match cmd {
            Command::Clock => self.handle_clock_cmd(),
            Command::MidiClock => self.handle_midi_cmd(&delta),
            Command::Event(event) => self.handle_event_cmd(event),
            Command::Track(num, rev, func) => self.handle_track_cmd(num, rev, func),
            Command::Stop => Ok(Status::Stop),
            _ => return Err(exception!()),
        });

        if let Status::Continue = status {
            Ok(Status::Continue)
        } else {
            self.midi.stop();
            self.handle_clock_cmd().ok();
            Ok(status)
        }
    }

    // Read internal and external commands
    fn handle_clock_cmd(&mut self) -> Result<Status, Error> {
        while let Some(cmd) = (self.input)() {
            match cmd {
                Command::Event(_) => (),
                Command::Stop => {
                    (self.clock)(TimeEvent::Stop);
                    return Ok(Status::Stop);
                }
                Command::Reload => {
                    (self.clock)(TimeEvent::Stop);
                    return Ok(Status::Reload);
                }
                Command::MidiNoteOn(_, _, _)
                | Command::MidiNoteOff(_, _)
                | Command::MidiCtl(_, _, _) => (self.sink)(cmd),
                _ => return Err(exception!()),
            };
        }
        Ok(Status::Continue)
    }

    // Route sequenced events
    fn handle_event_cmd(&mut self, event: Event) -> Result<Status, Error> {
        (self.sink)(Command::Event(event));
        match event.dest {
            Destination::Midi(_, _) => {
                self.midi.process(event);
                Ok(Status::Continue)
            }
        }
    }

    // Update midi messages (note, ctrl, clock etc.)
    fn handle_midi_cmd(&mut self, elapsed: &Duration) -> Result<Status, Error> {
        self.midi.update(elapsed);
        Ok(Status::Continue)
    }

    // Call a track function scheduling its produced events
    fn handle_track_cmd(&mut self, num: usize, rev: usize, func: u64) -> Result<Status, Error> {
        // Evaluate the track function
        self.interp.data_mut().reset(rev);
        self.interp.reset();
        try!(self.interp.eval(self.functions[&func]));

        // Apply effects
        let data = self.interp.data_mut();
        let track = &mut data.tracks[num];
        for fx in &mut track.effects {
            let fx = Rc::get_mut(fx).unwrap();
            data.events = fx.apply(data.duration, &data.events);
        }

        // Schedule events
        for event in &data.events {
            let cmd = TimeEvent::Timeout(event.onset, Command::Event(*event));
            (self.clock)(cmd);
        }

        // Re-schedule the track
        let cmd = Command::Track(num, rev + 1, func);
        (self.clock)(TimeEvent::Timeout(data.duration, cmd));
        Ok(Status::Continue)
    }
}
