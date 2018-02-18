mod fx;
mod interp;
mod math;
mod midi;
mod time;
mod types;
mod words;

use std::collections::HashMap;
use std::rc::Rc;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::thread;
use std::time::Duration;

use err::Error;
use lang::hash_str;

pub use self::interp::{Instr, InterpState, Value};
use self::interp::{BaseInterpreter, Interpreter, StackTraceInterpreter};
pub use self::math::{dur_to_millis, millis_to_dur};
use self::midi::MidiProcessor;
use self::time::{TimeEvent, TimerUnit};
pub use self::types::{Command, Destination, Event, EventValue};
use self::types::{SeqState, Track};

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum Control {
    Reload,
    Stop,
    Continue,
}

#[derive(Copy, Clone, Debug, PartialEq)]
enum Signal {
    Midi,
    Bus,
    Event(Event),
    Track(usize, usize, u64),
}

#[derive(Debug)]
struct SignalState {
    output: Sender<TimeEvent<Signal>>,
    input: Receiver<TimeEvent<Signal>>,
}

pub struct Machine {
    pub interp: Box<Interpreter<SeqState>>,
    backend: Sender<Command>,
    bus_recv: Receiver<Command>,
    functions: HashMap<u64, usize>,
    midi: MidiProcessor,
}

impl Machine {
    pub fn new(
        backend: Sender<Command>,
        bus_send: Sender<Command>,
        bus_recv: Receiver<Command>,
        instrs: &[Instr],
    ) -> Machine {
        let mut funcs = HashMap::new();
        for (pc, instr) in instrs.iter().enumerate() {
            if let Instr::Begin(word) = *instr {
                funcs.insert(word, pc + 1);
            }
        }

        let interp = Box::new(BaseInterpreter::new(
            instrs.to_vec(),
            &words::all(),
            SeqState::new(),
        ));

        let interp = Box::new(StackTraceInterpreter::new(interp));

        Machine {
            backend: backend,
            bus_recv: bus_recv,
            functions: funcs,
            interp: interp,
            midi: MidiProcessor::new(bus_send.clone()),
        }
    }

    pub fn exec(&mut self, duration: Duration, delta: Duration) -> Result<Control, Error> {
        let (mut signals, mut timers) = try!(self.setup());
        let mut elapsed = Duration::new(0, 0);

        while elapsed < duration {
            while let Ok(cmd) = signals.input.try_recv() {
                let status = try!(self.handle_signal(cmd, &mut signals));
                match status {
                    Control::Continue => continue,
                    _ => {
                        self.flush(&mut signals);
                        return Ok(status);
                    }
                }
            }

            timers.tick(&delta);
            elapsed += delta;
        }

        self.flush(&mut signals);
        Ok(Control::Stop)
    }

    pub fn exec_realtime(&mut self) -> Result<Control, Error> {
        let (mut signals, mut timers) = try!(self.setup());
        if self.interp.data_mut().tracks.is_empty() {
            return Ok(Control::Stop);
        }

        let handle = thread::spawn(move || timers.run_forever());

        while let Ok(cmd) = signals.input.recv() {
            let status = try!(self.handle_signal(cmd, &mut signals));
            match status {
                Control::Continue => continue,
                _ => {
                    handle.join().ok();
                    self.flush(&mut signals);
                    return Ok(status);
                }
            }
        }

        self.flush(&mut signals);
        Ok(Control::Continue)
    }

    pub fn eval(&mut self, func: &str, rev: usize) -> Result<Value, Error> {
        let func = hash_str(func);
        {
            let data = self.interp.data_mut();
            data.revision = rev;
            data.duration = 0.0;
            data.events.clear();
        }
        self.interp.reset();

        match self.interp.eval(self.functions[&func]) {
            Err(err) => Err(err),
            Ok(val) => Ok(match val {
                Some(val) => val,
                None => Value::Null,
            }),
        }
    }

    fn setup(&mut self) -> Result<(SignalState, TimerUnit<Signal>), Error> {
        let (timer_to_vm_send, timer_to_vm_recv) = channel();
        let (vm_to_timer_send, vm_to_timer_recv) = channel();
        let signals = SignalState {
            output: vm_to_timer_send.clone(),
            input: timer_to_vm_recv,
        };

        // Setup timers, schduling the recurring signals (ms)
        let mut timers = TimerUnit::new(timer_to_vm_send, vm_to_timer_recv);
        timers.interval(2.0, Signal::Midi);
        timers.interval(0.5, Signal::Bus);

        // Create tracks as defined by block 1
        if let Some(val) = try!(self.interp.eval_block(1)) {
            let (start, end) = try!(val.as_range());
            let state = self.interp.state();
            for (i, ptr) in (start..end).enumerate() {
                let sym = try!(try!(state.heap_get(ptr)).as_sym());
                let data = self.interp.data_mut();
                data.tracks.push(Track::new(i, sym));
            }
        }

        // Reset interpreter and call into `main`
        self.interp.data_mut().reset(0);
        self.interp.reset();
        match self.functions.get(&hash_str("main")) {
            Some(pc) => try!(self.interp.eval(*pc)),
            None => None,
        };

        // Schedule track functions to be interpreted
        for track in &self.interp.data_mut().tracks {
            let track = Signal::Track(track.id, 0, track.func);
            let cmd = TimeEvent::Timeout(0.0, track);
            signals.output.send(cmd).ok();
        }

        Ok((signals, timers))
    }

    fn flush(&mut self, signals: &mut SignalState) {
        self.midi.stop();
        self.handle_bus_signal(signals).ok();
    }

    // Main signal handler
    fn handle_signal(
        &mut self,
        cmd: TimeEvent<Signal>,
        signals: &mut SignalState,
    ) -> Result<Control, Error> {
        match cmd {
            TimeEvent::Timer(time, signal) => match signal {
                Signal::Bus => self.handle_bus_signal(signals),
                Signal::Midi => self.handle_midi_signal(&time),
                Signal::Event(event) => self.handle_event_signal(event),
                Signal::Track(num, rev, func) => self.handle_track_signal(signals, num, rev, func),
            },
            _ => Err(exception!()),
        }
    }

    // Read internal and external commands
    fn handle_bus_signal(&mut self, signals: &mut SignalState) -> Result<Control, Error> {
        while let Ok(msg) = self.bus_recv.try_recv() {
            match msg {
                Command::Event(_) => (),

                Command::Stop => {
                    signals.output.send(TimeEvent::Stop).ok();
                    return Ok(Control::Stop);
                }

                Command::Reload => {
                    signals.output.send(TimeEvent::Stop).ok();
                    return Ok(Control::Reload);
                }

                Command::MidiNoteOn(_, _, _)
                | Command::MidiNoteOff(_, _)
                | Command::MidiCtl(_, _, _) => {
                    if self.backend.send(msg).is_err() {
                        return Err(error!(UnreachableBackend));
                    }
                }
            };
        }
        Ok(Control::Continue)
    }

    // Route sequenced events
    fn handle_event_signal(&mut self, event: Event) -> Result<Control, Error> {
        if self.backend.send(Command::Event(event)).is_err() {
            return Err(error!(UnreachableBackend));
        }

        match event.dest {
            Destination::Midi(_, _) => {
                self.midi.process(event);
                Ok(Control::Continue)
            }
        }
    }

    // Update midi messages (note, ctrl, clock etc.)
    fn handle_midi_signal(&mut self, elapsed: &Duration) -> Result<Control, Error> {
        self.midi.update(elapsed);
        Ok(Control::Continue)
    }

    // Call a track function scheduling its produced events
    fn handle_track_signal(
        &mut self,
        signals: &mut SignalState,
        num: usize,
        rev: usize,
        func: u64,
    ) -> Result<Control, Error> {
        self.interp.data_mut().reset(rev);
        self.interp.reset();
        try!(self.interp.eval(self.functions[&func]));

        // Apply track effects
        let data = self.interp.data_mut();
        let track = &mut data.tracks[num];
        for fx in &mut track.effects {
            let fx = Rc::get_mut(fx).unwrap();
            data.events = fx.apply(data.duration, &data.events);
        }

        for event in &data.events {
            let cmd = TimeEvent::Timeout(event.onset, Signal::Event(*event));
            signals.output.send(cmd).ok();
        }

        let msg = Signal::Track(num, rev + 1, func);
        signals
            .output
            .send(TimeEvent::Timeout(data.duration, msg))
            .ok();
        Ok(Control::Continue)
    }
}
