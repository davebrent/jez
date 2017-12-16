mod audio;
mod filters;
mod interp;
mod markov;
mod math;
mod msgs;
mod midi;
mod pitch;
mod ring;
mod synths;
mod time;
mod words;

use std::collections::HashMap;
use std::convert::From;
use std::rc::Rc;
use std::sync::mpsc::{Receiver, Sender, channel};
use std::thread;
use std::time::Duration;

use err::{JezErr, RuntimeErr, SysErr};
use lang::hash_str;

pub use self::audio::AudioBlock;
use self::audio::AudioProcessor;
pub use self::interp::Instr;
use self::interp::Interpreter;
pub use self::math::millis_to_dur;
use self::midi::MidiProcessor;
pub use self::msgs::{Command, Destination, Event, EventValue};
pub use self::ring::RingBuffer;
use self::time::{TimeEvent, TimerUnit};
use self::words::{ExtKeyword, ExtState, bin_list, block_size, channels, cycle,
                  degrade, every, gray_code, hop_jump, inter_onset,
                  intersection, linear, markov_filter, midi_out, onsets,
                  palindrome, pitch_quantize_filter, rand_range, rand_seed,
                  range, repeat, reverse, revision, rotate, sample_rate,
                  shuffle, sieve, simul, symmetric_difference, synth_out,
                  tracks, union, wave_table};

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum Control {
    Reload,
    Stop,
    Continue,
}

#[derive(Copy, Clone, Debug, PartialEq)]
enum Signal {
    Audio,
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
    backend: Sender<Command>,
    bus_recv: Receiver<Command>,
    functions: HashMap<u64, usize>,
    interp: Interpreter<ExtState>,
    midi: MidiProcessor,
    audio: AudioProcessor,
}

impl Machine {
    pub fn new(ring: RingBuffer<AudioBlock>,
               backend: Sender<Command>,
               bus_send: Sender<Command>,
               bus_recv: Receiver<Command>,
               instrs: &[Instr])
               -> Machine {
        let mut funcs = HashMap::new();
        for (pc, instr) in instrs.iter().enumerate() {
            if let Instr::Begin(word) = *instr {
                funcs.insert(word, pc + 1);
            }
        }

        let mut words: HashMap<&'static str, ExtKeyword> = HashMap::new();
        words.insert("bin_list", bin_list);
        words.insert("cycle", cycle);
        words.insert("degrade", degrade);
        words.insert("every", every);
        words.insert("gray_code", gray_code);
        words.insert("hop_jump", hop_jump);
        words.insert("linear", linear);
        words.insert("palindrome", palindrome);
        words.insert("repeat", repeat);
        words.insert("revision", revision);
        words.insert("reverse", reverse);
        words.insert("rotate", rotate);
        words.insert("shuffle", shuffle);
        words.insert("simul", simul);
        words.insert("midi_out", midi_out);
        words.insert("tracks", tracks);
        words.insert("channels", channels);
        words.insert("block_size", block_size);
        words.insert("sample_rate", sample_rate);
        words.insert("wave_table", wave_table);
        words.insert("synth_out", synth_out);
        words.insert("rand_seed", rand_seed);
        words.insert("rand_range", rand_range);
        words.insert("markov_filter", markov_filter);
        words.insert("pitch_quantize_filter", pitch_quantize_filter);
        words.insert("range", range);
        words.insert("sieve", sieve);
        words.insert("intersection", intersection);
        words.insert("union", union);
        words.insert("symmetric_difference", symmetric_difference);
        words.insert("inter_onset", inter_onset);
        words.insert("onsets", onsets);

        Machine {
            backend: backend,
            bus_recv: bus_recv,
            functions: funcs,
            interp: Interpreter::new(instrs.to_vec(), words, ExtState::new()),
            midi: MidiProcessor::new(bus_send.clone()),
            audio: AudioProcessor::new(ring, bus_send.clone()),
        }
    }

    pub fn exec(&mut self,
                duration: Duration,
                delta: Duration)
                -> Result<Control, JezErr> {
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

    pub fn exec_realtime(&mut self) -> Result<Control, JezErr> {
        let (mut signals, mut timers) = try!(self.setup());
        if self.interp.data.tracks.is_empty() {
            return Ok(Control::Stop);
        }

        let handle = thread::spawn(move || {
            let res = Duration::new(0, 1_000_000);
            timers.run_forever(res);
        });

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

    fn setup(&mut self) -> Result<(SignalState, TimerUnit<Signal>), JezErr> {
        let (timer_to_vm_send, timer_to_vm_recv) = channel();
        let (vm_to_timer_send, vm_to_timer_recv) = channel();
        let signals = SignalState {
            output: vm_to_timer_send.clone(),
            input: timer_to_vm_recv,
        };

        // Setup timers, schduling the recurring signals (ms)
        let mut timers = TimerUnit::new(timer_to_vm_send, vm_to_timer_recv);
        timers.interval(3.0, Signal::Midi);
        timers.interval(2.0, Signal::Bus);

        // Reset interpreter and call into `main`
        self.interp.data.revision = 0;
        self.interp.data.duration = 0.0;
        self.interp.data.events.clear();
        self.interp.state.reset();
        try!(self.interp.eval(self.functions[&hash_str("main")]));

        // Allow `main` to override default audio settings and schedule the
        // signal at the desired audio rate
        self.audio.configure(self.interp.data.audio.clone());
        self.interp.data.audio.synths.clear();
        timers.interval(self.audio.interval(), Signal::Audio);

        // Schedule track functions to be interpreted
        for track in &self.interp.data.tracks {
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
    fn handle_signal(&mut self,
                     cmd: TimeEvent<Signal>,
                     signals: &mut SignalState)
                     -> Result<Control, JezErr> {
        match cmd {
            TimeEvent::Timer(time, signal) => {
                match signal {
                    Signal::Audio => self.handle_audio_signal(&time),
                    Signal::Bus => self.handle_bus_signal(signals),
                    Signal::Midi => self.handle_midi_signal(&time),
                    Signal::Event(event) => self.handle_event_signal(event),
                    Signal::Track(num, rev, func) => {
                        self.handle_track_signal(signals, num, rev, func)
                    }
                }
            }
            _ => Err(From::from(RuntimeErr::InvalidArgs)),
        }
    }

    // Update audio processor
    fn handle_audio_signal(&mut self,
                           elapsed: &Duration)
                           -> Result<Control, JezErr> {
        self.audio.update(elapsed);
        Ok(Control::Continue)
    }

    // Read internal and external commands
    fn handle_bus_signal(&mut self,
                         signals: &mut SignalState)
                         -> Result<Control, JezErr> {
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

                Command::AudioSettings(_, _, _) |
                Command::MidiNoteOn(_, _, _) |
                Command::MidiNoteOff(_, _) |
                Command::MidiCtl(_, _, _) => {
                    if self.backend.send(msg).is_err() {
                        return Err(From::from(SysErr::UnreachableBackend));
                    }
                }
            };
        }
        Ok(Control::Continue)
    }

    // Route sequenced events
    fn handle_event_signal(&mut self, event: Event) -> Result<Control, JezErr> {
        if self.backend.send(Command::Event(event)).is_err() {
            return Err(From::from(SysErr::UnreachableBackend));
        }

        match event.dest {
            Destination::Midi(_, _) => {
                self.midi.process(event);
                Ok(Control::Continue)
            }
            Destination::Synth(_, _) => {
                self.audio.process(event);
                Ok(Control::Continue)
            }
        }
    }

    // Update midi messages (note, ctrl, clock etc.)
    fn handle_midi_signal(&mut self,
                          elapsed: &Duration)
                          -> Result<Control, JezErr> {
        self.midi.update(elapsed);
        Ok(Control::Continue)
    }

    // Call a track function scheduling its produced events
    fn handle_track_signal(&mut self,
                           signals: &mut SignalState,
                           num: usize,
                           rev: usize,
                           func: u64)
                           -> Result<Control, JezErr> {
        self.interp.data.revision = rev;
        self.interp.data.duration = 0.0;
        self.interp.data.events.clear();
        self.interp.state.reset();
        try!(self.interp.eval(self.functions[&func]));

        // Apply the tracks filters
        let track = &mut self.interp.data.tracks[num];
        let dur = self.interp.data.duration;
        for filter in &mut track.filters {
            let f = try!(Rc::get_mut(filter).ok_or(JezErr::RuntimeErr(
                RuntimeErr::InvalidArgs,
            )));
            self.interp.data.events = f.apply(dur, &self.interp.data.events);
        }

        // Avoid recursive scheduling of this handler
        let dur = self.interp.data.duration;
        if dur == 0.0 {
            return Err(From::from(RuntimeErr::InvalidArgs));
        }

        for event in &self.interp.data.events {
            let cmd = TimeEvent::Timeout(event.onset, Signal::Event(*event));
            signals.output.send(cmd).ok();
        }

        let msg = Signal::Track(num, rev + 1, func);
        signals.output.send(TimeEvent::Timeout(dur, msg)).ok();
        Ok(Control::Continue)
    }
}
