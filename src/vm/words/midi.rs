use crate::vm::interp::{InterpState, Value};
use crate::vm::types::{Destination, Event, EventValue, Result, SeqState};

/// Output midi events
pub fn midi_out(seq: &mut SeqState, state: &mut InterpState) -> Result {
    let chan = state.pop_num()? as u8;
    let dur = state.pop_num()?;
    if dur == 0.0 {
        return Err(error!(InvalidArgs));
    }

    let mut output = Vec::new();

    let mut visit: Vec<(f64, f64, Value)> = Vec::new();
    visit.push((0.0, dur, state.pop()?));

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
            Value::Seq(start, end) => {
                let interval = dur / (end - start) as f64;
                let mut onset = onset;
                for n in start..end {
                    visit.push((onset, interval, state.heap_get(n)?));
                    onset += interval;
                }
            }
            Value::Group(start, end) => {
                for n in start..end {
                    visit.push((onset, dur, state.heap_get(n)?));
                }
            }
            Value::List(start, end) => {
                let len = end - start;
                if len == 0 || len > 3 {
                    return Err(error!(InvalidArgs));
                }

                let (value, default) = match state.heap_get(start)? {
                    Value::Curve(points) => (EventValue::Curve(points), 0),
                    Value::Number(pitch) => (EventValue::Trigger(pitch), 127),
                    _ => return Err(error!(InvalidArgs)),
                };

                let dest = Destination::Midi(
                    if len == 3 {
                        (state.heap_get(start + 2)?).as_num()? as u8
                    } else {
                        chan
                    },
                    if len == 2 {
                        (state.heap_get(start + 1)?).as_num()? as u8
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
            _ => return Err(error!(InvalidArgs)),
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
        state.call(0, 0, 1).unwrap();
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
