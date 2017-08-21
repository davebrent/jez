extern crate jez;
use std::time::Duration;
use jez::{Command, Simulation};

fn filter_midi_notes(sim: &Simulation, on: bool) -> Vec<Command> {
    sim.messages
        .iter()
        .map(|l| l.data)
        .filter(|&msg| match msg {
                    Command::MidiNoteOn(_, _, _) => on,
                    Command::MidiNoteOff(_, _) => !on,
                    _ => false,
                })
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
  [64 66 68 70] 250 1 127 midiout

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
