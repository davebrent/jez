mod fx;
mod handler;
mod interp;
mod math;
mod time;
mod types;
mod words;

use std::collections::HashMap;

use crate::err::Error;
use crate::lang::hash_str;

use self::handler::{EventHandler, NoteInterceptor};
use self::interp::{BaseInterpreter, Interpreter, StackTraceInterpreter};
pub use self::interp::{Instr, InterpState, Value};
use self::time::Clock as InternalClock;
pub use self::time::{millis_to_dur, Schedule};
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
) -> Result<(HashMap<u64, usize>, Box<dyn Interpreter<SeqState>>), Error> {
    let mut interp = Box::new(StackTraceInterpreter::new(Box::new(BaseInterpreter::new(
        instrs.to_vec(),
        &words::all(),
        SeqState::new(),
    ))));

    // Create tracks as defined by block 1 (the extension block)
    if let Some(val) = interp.eval_block(1)? {
        let (start, end) = val.as_range()?;
        let state = interp.state();
        for (i, ptr) in (start..end).enumerate() {
            let sym = (state.heap_get(ptr)?).as_sym()?;
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
            interp.eval(pc)?;
        }
        None => (),
    };

    // Reset the interpreter again for first use
    interp.data_mut().reset(0);
    interp.reset();
    Ok((funcs, interp))
}

type Timer = Box<dyn FnMut(Schedule<Command>)>;
type In = Box<dyn FnMut() -> Option<Command>>;
type Out = Box<dyn FnMut(Command)>;

pub struct Machine {
    interp: Box<dyn Interpreter<SeqState>>,
    clock: Timer,
    sink: Out,
    input: In,
    functions: HashMap<u64, usize>,
    handler: EventHandler,
}

impl Machine {
    pub fn new(input: In, sink: Out, clock: Timer, instrs: &[Instr]) -> Result<Machine, Error> {
        let (funcs, mut interp) = self::interpreter(instrs)?;
        let mut cmds = vec![];

        for track in &interp.data_mut().tracks {
            cmds.push(Command::Track(track.id, 0, track.func));
        }

        let mut note_interceptor = NoteInterceptor::new(sink);
        let mut machine = Machine {
            sink: Box::new(move |cmd| {
                note_interceptor.filter(cmd);
            }),
            clock: clock,
            input: input,
            functions: funcs,
            interp: interp,
            handler: EventHandler::new(),
        };

        for cmd in &cmds {
            machine.process(*cmd)?;
        }

        Ok(machine)
    }

    pub fn process(&mut self, cmd: Command) -> Result<Status, Error> {
        let status = match cmd {
            Command::Stop => Ok(Status::Stop),
            Command::Reload => Ok(Status::Reload),
            Command::Clock => self.handle_clock_cmd(),
            Command::Track(num, rev, func) => self.handle_track_cmd(num, rev, func),
            _ => {
                (self.sink)(cmd);
                Ok(Status::Continue)
            }
        }?;

        if let Status::Continue = status {
            Ok(Status::Continue)
        } else {
            Ok(status)
        }
    }

    fn handle_clock_cmd(&mut self) -> Result<Status, Error> {
        if let Some(cmd) = (self.input)() {
            match cmd {
                Command::Stop => {
                    (self.clock)(Schedule::Stop);
                    return Ok(Status::Stop);
                }
                Command::Reload => {
                    (self.clock)(Schedule::Stop);
                    return Ok(Status::Reload);
                }
                _ => return Err(exception!()),
            };
        }
        Ok(Status::Continue)
    }

    fn handle_track_cmd(&mut self, num: usize, rev: usize, func: u64) -> Result<Status, Error> {
        self.interp.data_mut().reset(rev);
        self.interp.reset();
        self.interp.eval(self.functions[&func])?;

        let data = self.interp.data_mut();
        let track = &mut data.tracks[num];

        for fx in &mut track.effects {
            data.events = fx.apply(data.duration, &data.events);
        }

        for event in &mut data.events {
            event.onset += track.real_time;
            self.handler.handle(&mut self.clock, *event);
        }

        // Tracks are scheduled one revision _ahead_ of the clock
        track.real_time += data.duration;
        track.schedule_time += if rev == 0 { 0.0 } else { data.duration };
        let cmd = Command::Track(num, rev + 1, func);
        (self.clock)(Schedule::At(track.schedule_time, cmd));
        Ok(Status::Continue)
    }
}
