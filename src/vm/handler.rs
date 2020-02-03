use super::math::{point_on_curve, Curve};
use super::time::Schedule;
use super::types::{Command, Destination, Event, EventValue};

type Clock = Box<dyn FnMut(Schedule<Command>)>;
type Out = Box<dyn FnMut(Command)>;

pub struct EventHandler;

pub struct NoteInterceptor {
    output: Out,
    pending: Vec<(u8, u8)>,
}

impl NoteInterceptor {
    pub fn new(output: Out) -> NoteInterceptor {
        NoteInterceptor {
            output: output,
            pending: vec![],
        }
    }

    pub fn filter(&mut self, cmd: Command) {
        match cmd {
            Command::MidiNoteOn(channel, pitch, _) => {
                self.pending.push((channel, pitch));
                (self.output)(cmd);
            }
            Command::MidiNoteOff(channel, pitch) => {
                self.pending
                    .retain(|&evt| !(evt.0 == channel && evt.1 == pitch));
                (self.output)(cmd);
            }
            Command::Stop => {
                for &(channel, pitch) in &self.pending {
                    (self.output)(Command::MidiNoteOff(channel, pitch));
                }
                (self.output)(cmd);
            }
            _ => (self.output)(cmd),
        }
    }
}

impl EventHandler {
    pub fn new() -> EventHandler {
        EventHandler {}
    }

    pub fn handle(&mut self, output: &mut Clock, event: Event) {
        match event.value {
            EventValue::Trigger(val) => self.handle_trigger(output, event, val as u8),
            EventValue::Curve(curve) => self.handle_control(output, event, curve),
        };
    }

    fn handle_trigger(&mut self, output: &mut Clock, event: Event, val: u8) {
        let (chan, vel) = match event.dest {
            Destination::Midi(chan, vel) => (chan, vel),
        };

        let cmd = Command::Event(event);
        output(Schedule::At(event.onset, cmd));
        let cmd = Command::MidiNoteOn(chan, val, vel);
        output(Schedule::At(event.onset, cmd));
        let cmd = Command::MidiNoteOff(chan, val);
        output(Schedule::At(event.onset + event.dur, cmd));
    }

    fn handle_control(&mut self, output: &mut Clock, event: Event, val: Curve) {
        let cmd = Command::Event(event);
        output(Schedule::At(event.onset, cmd));

        let (chan, ctl) = match event.dest {
            Destination::Midi(chan, ctl) => (chan, ctl),
        };

        let mut elapsed = 0.0;
        let mut previous = None;
        let delta = 1000.0 / 125.0; // target messages per second (roughly)

        while elapsed <= event.dur {
            let t = elapsed / event.dur;
            let cc = point_on_curve(t, &val)[1].round() as u8;

            if previous != Some(cc) {
                let cmd = Command::MidiCtl(chan, ctl, cc);
                output(Schedule::At(event.onset + elapsed, cmd));
                previous = Some(cc);
            }

            elapsed += delta;
        }
    }
}
