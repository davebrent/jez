use std::time::Duration;

use super::math::{dur_to_millis, millis_to_dur, point_on_curve, Curve};
use super::types::{Command, Destination, Event, EventValue};

#[derive(Copy, Clone, Debug, PartialEq)]
struct CtrlState {
    duration: Duration,
    t: f64,
    channel: u8,
    controller: u8,
    curve: Curve,
    previous: u8,
}

#[derive(Copy, Clone, Debug, PartialEq)]
struct NoteState {
    duration: Duration,
    channel: u8,
    pitch: u8,
}

pub struct MidiProcessor {
    output: Box<FnMut(Command)>,
    off_events: Vec<NoteState>,
    ctl_events: Vec<CtrlState>,
    last_update: Duration,
}

impl MidiProcessor {
    pub fn new(output: Box<FnMut(Command)>) -> MidiProcessor {
        MidiProcessor {
            output: output,
            off_events: Vec::new(),
            ctl_events: Vec::new(),
            last_update: Duration::new(0, 0),
        }
    }

    pub fn update(&mut self, elapsed: &Duration) {
        let delta = match elapsed.checked_sub(self.last_update) {
            Some(dur) => dur,
            None => Duration::new(0, 0),
        };
        self.last_update = *elapsed;
        self.update_ctl_events(&delta);
        self.update_off_events(&delta);
    }

    pub fn stop(&mut self) {
        while let Some(note) = self.off_events.pop() {
            let cmd = Command::MidiNoteOff(note.channel, note.pitch);
            (self.output)(cmd);
        }
    }

    pub fn process(&mut self, event: Event) {
        match event.value {
            EventValue::Trigger(val) => self.handle_trg_event(event, val as u8),
            EventValue::Curve(curve) => self.handle_ctl_event(event, curve),
        };
    }

    fn handle_trg_event(&mut self, event: Event, ptch: u8) {
        let (chan, vel) = match event.dest {
            Destination::Midi(chan, vel) => (chan, vel),
        };

        let len = self.off_events.len();
        self.off_events
            .retain(|&evt| !(evt.channel == chan && evt.pitch == ptch));
        if len != self.off_events.len() {
            (self.output)(Command::MidiNoteOff(chan, ptch));
        }

        self.off_events.push(NoteState {
            duration: millis_to_dur(event.dur),
            channel: chan,
            pitch: ptch,
        });
        self.off_events
            .sort_by(|a, b| b.duration.partial_cmp(&a.duration).unwrap());
        (self.output)(Command::MidiNoteOn(chan, ptch, vel));
    }

    fn handle_ctl_event(&mut self, event: Event, curve: Curve) {
        let (chan, ctl) = match event.dest {
            Destination::Midi(chan, vel) => (chan, vel),
        };

        let initial = point_on_curve(0.0, &curve)[1].round() as u8;
        let existing = self.ctl_events
            .iter()
            .position(|&evt| evt.channel == chan && evt.controller == ctl);

        let send_init = match existing {
            None => true,
            Some(index) => {
                let send = initial != self.ctl_events[index].previous;
                self.ctl_events.remove(index);
                send
            }
        };

        self.ctl_events.push(CtrlState {
            t: 0.0,
            duration: millis_to_dur(event.dur),
            channel: chan,
            controller: ctl,
            curve: curve,
            previous: initial,
        });

        if send_init {
            let cmd = Command::MidiCtl(chan, ctl, initial);
            (self.output)(cmd);
        }
    }

    fn update_ctl_events(&mut self, delta: &Duration) {
        for evt in &mut self.ctl_events {
            evt.t += dur_to_millis(delta) / dur_to_millis(&evt.duration);
        }

        for evt in &mut self.ctl_events {
            let cc = point_on_curve(evt.t, &evt.curve)[1].round() as u8;
            if cc != evt.previous {
                evt.previous = cc;
                let cmd = Command::MidiCtl(evt.channel, evt.controller, cc);
                (self.output)(cmd);
            }
        }

        self.ctl_events.retain(|&evt| evt.t < 1.0);
    }

    fn update_off_events(&mut self, delta: &Duration) {
        let zero = Duration::new(0, 0);

        for evt in &mut self.off_events {
            evt.duration = match evt.duration.checked_sub(*delta) {
                Some(dur) => dur,
                None => zero,
            }
        }

        while let Some(note) = self.off_events.pop() {
            if note.duration != zero {
                self.off_events.push(note);
                break;
            } else {
                let cmd = Command::MidiNoteOff(note.channel, note.pitch);
                (self.output)(cmd);
            }
        }
    }
}
