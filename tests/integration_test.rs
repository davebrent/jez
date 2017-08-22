extern crate jez;
use std::time::Duration;
use jez::{Command, Destination, Event, EventValue, Simulation, LogData};

fn filter_commands(sim: &Simulation) -> Vec<Command> {
    let out: Vec<Command> = Vec::new();
    sim.messages.iter().fold(out, |mut out, msg| {
        if let LogData::Command(cmd) = msg.data {
            out.push(cmd);
        }
        out
    })
}

fn filter_events(sim: &Simulation) -> Vec<Event> {
    let out: Vec<Event> = Vec::new();
    sim.messages.iter().fold(out, |mut out, msg| {
        if let LogData::Event(evt) = msg.data {
            out.push(evt);
        }
        out
    })
}

fn filter_midi_notes(sim: &Simulation, on: bool) -> Vec<Command> {
    filter_commands(sim)
        .iter()
        .filter(|&cmd| match *cmd {
            Command::MidiNoteOn(_, _, _) => on,
            Command::MidiNoteOff(_, _) => !on,
            _ => false
        })
        .cloned()
        .collect()
}

#[test]
fn test_simple_program() {
    let dur = Duration::new(0, 250_000_000);
    let dt = Duration::new(0, 1_000_000);
    let res = jez::simulate(dur,
                            dt,
                            "
.version 1

.def t1 0:
  [64 66 68 70] 250 1 127 midi_out

.def main 0:
  ['t1] tracks
    ");

    let sim = res.unwrap();
    let on = filter_midi_notes(&sim, true);
    let off = filter_midi_notes(&sim, false);

    assert_eq!(on,
               vec![Command::MidiNoteOn(1, 64, 127),
                    Command::MidiNoteOn(1, 66, 127),
                    Command::MidiNoteOn(1, 68, 127),
                    Command::MidiNoteOn(1, 70, 127)]);

    assert_eq!(off,
               vec![Command::MidiNoteOff(1, 64),
                    Command::MidiNoteOff(1, 66),
                    Command::MidiNoteOff(1, 68),
                    Command::MidiNoteOff(1, 70)]);
}

#[test]
fn test_log_events() {
    let dur = Duration::new(0, 250_000_000);
    let dt = Duration::new(0, 1_000_000);
    let res = jez::simulate(dur,
                            dt,
                            "
.version 1

.def t1 0:
  [64 ~ 68 ~ 48] 250 1 127 midi_out

.def main 0:
  ['t1] tracks
    ");

    let sim = res.unwrap();
    let events = filter_events(&sim);

    let a = Event{
        dest: Destination::Midi(1, 127),
        onset: 0.0,
        dur: 50.0,
        value: EventValue::Trigger(64.0),
    };

    let b = Event{
        dest: Destination::Midi(1, 127),
        onset: 100.0,
        dur: 50.0,
        value: EventValue::Trigger(68.0),
    };

    let c = Event{
        dest: Destination::Midi(1, 127),
        onset: 200.0,
        dur: 50.0,
        value: EventValue::Trigger(48.0),
    };

    assert_eq!(events, vec![a, b, c]);
}
