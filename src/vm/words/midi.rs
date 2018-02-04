use err::RuntimeErr;

use vm::interp::{InterpState, Value};
use vm::types::{Destination, Event, EventValue, Result, SeqState};


/// Output midi events
pub fn midi_out(seq: &mut SeqState, state: &mut InterpState) -> Result {
    let chan = try!(state.pop_num()) as u8;
    let dur = try!(state.pop_num());

    let mut output = Vec::new();

    let mut visit: Vec<(f64, f64, Value)> = Vec::new();
    visit.push((0.0, dur, try!(state.pop())));

    while let Some((onset, dur, val)) = visit.pop() {
        match val {
            Value::Curve(points) => {
                let event = Event {
                    dest: Destination::Midi(chan, 0),
                    onset: onset,
                    dur: dur,
                    value: EventValue::Curve(points),
                };
                output.push(event);
            }
            Value::Null => (),
            Value::Number(val) => {
                let event = Event {
                    dest: Destination::Midi(chan, 127),
                    onset: onset,
                    dur: dur,
                    value: EventValue::Trigger(val),
                };
                output.push(event);
            }
            Value::Expr(start, end) => {
                let interval = dur / (end - start) as f64;
                let mut onset = onset;
                for n in start..end {
                    visit.push((onset, interval, try!(state.heap_get(n))));
                    onset += interval;
                }
            }
            Value::Group(start, end) => {
                for n in start..end {
                    visit.push((onset, dur, try!(state.heap_get(n))));
                }
            }
            Value::Pair(start, end) => {
                let len = end - start;
                if len == 0 || len > 3 {
                    return Err(RuntimeErr::InvalidArgs);
                }

                let (value, default) = match try!(state.heap_get(start)) {
                    Value::Curve(points) => (EventValue::Curve(points), 0),
                    Value::Number(pitch) => (EventValue::Trigger(pitch), 127),
                    _ => return Err(RuntimeErr::InvalidArgs),
                };

                let dest = Destination::Midi(
                    if len == 3 {
                        try!(try!(state.heap_get(start + 2)).as_num()) as u8
                    } else {
                        chan
                    },
                    if len == 2 {
                        try!(try!(state.heap_get(start + 1)).as_num()) as u8
                    } else {
                        default
                    },
                );

                output.push(Event {
                    dest: dest,
                    onset: onset,
                    dur: dur,
                    value: value,
                });
            }
            _ => return Err(RuntimeErr::InvalidArgs),
        }
    }

    seq.duration = dur;
    seq.events.append(&mut output);
    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simultaneous_events() {
        let mut state = InterpState::new();
        let mut seq = SeqState::new();
        state.call(0, 1).unwrap();
        state.heap_push(Value::Number(1.0));
        state.heap_push(Value::Number(2.0));
        state.heap_push(Value::Number(3.0));
        state.push(Value::Group(0, 3)).unwrap();
        state.push(Value::Number(1000.0)).unwrap();
        state.push(Value::Number(0.0)).unwrap();
        midi_out(&mut seq, &mut state).unwrap();

        assert_eq!(
            seq.events,
            [
                Event {
                    dest: Destination::Midi(0, 127),
                    onset: 0.0,
                    dur: 1000.0,
                    value: EventValue::Trigger(3.0),
                },
                Event {
                    dest: Destination::Midi(0, 127),
                    onset: 0.0,
                    dur: 1000.0,
                    value: EventValue::Trigger(2.0),
                },
                Event {
                    dest: Destination::Midi(0, 127),
                    onset: 0.0,
                    dur: 1000.0,
                    value: EventValue::Trigger(1.0),
                },
            ]
        );
    }
}