use std::sync::mpsc::{channel, Receiver};
use std::time::Duration;
use std::thread;

use serde_json;

use err::Error;
use lang::{assemble, parser, Directive};
use vm::{millis_to_dur, Clock, Command, Instr, Machine as VmMachine, Schedule, Status};
use sinks::{factory, Backend, CompositeSink, Device, Sink as SinkTrait, ThreadedSink};

pub struct Sink {
    inner: Box<SinkTrait>,
}

impl Sink {
    pub fn new(requests: &[Backend]) -> Result<Sink, Error> {
        let mut sinks = vec![];
        for request in requests {
            let sink = try!(factory(request));
            sinks.push(sink);
        }
        let sink = Box::new(CompositeSink::new(sinks));
        Ok(Sink {
            inner: Box::new(ThreadedSink::new(sink)),
        })
    }

    pub fn name(&self) -> &str {
        self.inner.name()
    }

    pub fn devices(&self) -> Vec<Box<Device>> {
        self.inner.devices()
    }

    pub fn run_forever(&mut self, channel: Receiver<Command>) {
        self.inner.run_forever(channel)
    }

    pub fn process(&mut self, cmd: Command) {
        self.inner.process(cmd)
    }
}

type Input = Box<FnMut() -> Option<Command>>;
type Output = Box<FnMut(Command)>;

#[derive(Clone, Debug, PartialEq)]
pub struct Program {
    instrs: Vec<Instr>,
}

pub struct Machine {
    clock: Option<Clock>,
    machine: VmMachine,
    channel: Receiver<Schedule<Command>>,
}

impl Program {
    pub fn new(code: &str) -> Result<Program, Error> {
        let dirs = try!(parser(code));
        let instrs = try!(assemble(code, &dirs));
        Ok(Program { instrs: instrs })
    }
}

impl Machine {
    pub fn new(prog: &Program, input: Input, output: Output) -> Result<Machine, Error> {
        let (clock_to_mach_send, clock_to_mach_recv) = channel();
        let (mach_to_clock_send, mach_to_clock_recv) = channel();

        let mut clock = Clock::new(clock_to_mach_send, mach_to_clock_recv);
        clock.interval(1000.0, Command::Clock);

        let machine = try!(VmMachine::new(
            input,
            output,
            Box::new(move |evt| mach_to_clock_send.send(evt).unwrap_or(())),
            &prog.instrs,
        ));

        Ok(Machine {
            clock: Some(clock),
            machine: machine,
            channel: clock_to_mach_recv,
        })
    }

    pub fn schedule(&mut self, dur: f64, cmd: Command) {
        let clock = match self.clock {
            Some(ref mut clock) => clock,
            None => return,
        };
        clock.timeout(dur, cmd)
    }

    pub fn update(&mut self, delta: f64) -> Result<Status, Error> {
        let delta = millis_to_dur(delta);
        let clock = match self.clock {
            Some(ref mut clock) => clock,
            None => return Ok(Status::Stop),
        };

        while let Ok(event) = self.channel.try_recv() {
            if let Schedule::At(_, cmd) = event {
                let status = try!(self.machine.process(cmd));
                match status {
                    Status::Continue => (),
                    Status::Stop | Status::Reload => return Ok(status),
                };
            } else {
                return Err(exception!());
            }
        }

        clock.tick(delta);
        Ok(Status::Continue)
    }

    pub fn run_forever(&mut self) -> Result<Status, Error> {
        let mut clock = match self.clock.take() {
            Some(clock) => clock,
            None => return Ok(Status::Stop),
        };

        thread::spawn(move || clock.run_forever());

        while let Ok(event) = self.channel.recv() {
            if let Schedule::At(_, cmd) = event {
                let status = try!(self.machine.process(cmd));
                match status {
                    Status::Continue => (),
                    Status::Stop | Status::Reload => return Ok(status),
                };
            } else {
                return Err(exception!());
            }
        }

        Ok(Status::Continue)
    }
}

pub fn simulate(duration: f64, delta: f64, program: &str) -> Result<String, Error> {
    #[derive(Serialize)]
    struct Results<'a> {
        program: &'a str,
        duration: Duration,
        delta: Duration,
        directives: Vec<Directive<'a>>,
        instructions: Vec<Instr>,
        commands: Vec<Command>,
    }

    let (sender, receiver) = channel();
    let directives = try!(parser(program));
    let instructions = try!(assemble(program, &directives));
    let mut machine = try!(Machine::new(
        &Program {
            instrs: instructions.clone(),
        },
        Box::new(|| None),
        Box::new(move |cmd| sender.send(cmd).unwrap_or(()))
    ));

    machine.schedule(duration, Command::Stop);

    let mut commands = Vec::new();
    loop {
        let status = try!(machine.update(delta));
        while let Ok(cmd) = receiver.try_recv() {
            commands.push(cmd);
        }
        match status {
            Status::Continue => continue,
            Status::Stop | Status::Reload => break,
        };
    }

    let results = Results {
        program: program,
        duration: millis_to_dur(duration),
        delta: millis_to_dur(delta),
        directives: directives,
        instructions: instructions,
        commands: commands,
    };

    Ok(serde_json::to_string(&results).unwrap())
}
