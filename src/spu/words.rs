use rand::{Rng, StdRng};

use err::RuntimeErr;
use unit::{Event, EventValue, InterpState, InterpResult, Value};
use math::path_to_curve;

use super::seq::{SeqState, SeqTrack};


/// Repeat a value 'n' times
pub fn repeat(_: &mut SeqState, state: &mut InterpState) -> InterpResult {
    let times: Option<f64> = state.stack.pop().unwrap().into();
    let times = times.unwrap() as u32;
    let val = state.stack.pop().unwrap();
    for _ in 0..times {
        state.stack.push(val);
    }
    Ok(())
}

/// Put a value on the stack every 'n' cycles
pub fn every(seq: &mut SeqState, state: &mut InterpState) -> InterpResult {
    let freq: Option<f64> = state.stack.pop().unwrap().into();
    let freq = freq.unwrap() as usize;
    if freq % seq.cycle.rev == 0 {
        state.stack.pop().unwrap();
    } else {
        // Remove the else clause from the stack
        let val = state.stack.pop().unwrap();
        state.stack.pop().unwrap();
        state.stack.push(val);
    }
    Ok(())
}

/// Reverse a list, leaving it on the stack
pub fn reverse(_: &mut SeqState, state: &mut InterpState) -> InterpResult {
    match *state.stack.last().unwrap() {
        Value::Pair(start, end) => {
            state.heap[start..end].reverse();
            Ok(())
        }
        _ => Err(RuntimeErr::InvalidArgs),
    }
}

/// Shuffle a list, leaving it on the stack
pub fn shuffle(_: &mut SeqState, state: &mut InterpState) -> InterpResult {
    match *state.stack.last().unwrap() {
        Value::Pair(start, end) => {
            let mut rng = StdRng::new().unwrap();
            rng.shuffle(&mut state.heap[start..end]);
            Ok(())
        }
        _ => Err(RuntimeErr::InvalidArgs),
    }
}

/// Rotate a list
pub fn rotate(_: &mut SeqState, state: &mut InterpState) -> InterpResult {
    let amount: Option<f64> = state.stack.pop().unwrap().into();
    let amount = amount.unwrap() as usize;
    match *state.stack.last().unwrap() {
        Value::Pair(start, end) => {
            let lst = &state.heap[start..end].to_vec();
            let (a, b) = lst.split_at(lst.len() - amount);
            let mut out = Vec::new();
            out.extend_from_slice(b);
            out.extend_from_slice(a);
            state.heap[start..end].copy_from_slice(&out);
            Ok(())
        }
        _ => Err(RuntimeErr::InvalidArgs),
    }
}

/// Randomly set values to rests in a list
pub fn degrade(_: &mut SeqState, state: &mut InterpState) -> InterpResult {
    let mut rng = StdRng::new().unwrap();
    match *state.stack.last().unwrap() {
        Value::Pair(start, end) => {
            let lst = &mut state.heap[start..end];
            for item in lst {
                if rng.gen() {
                    *item = Value::Null;
                }
            }
            Ok(())
        }
        _ => Err(RuntimeErr::InvalidArgs),
    }
}

/// Every cycle, puts the 'next' element of a list on the stack
pub fn cycle(seq: &mut SeqState, state: &mut InterpState) -> InterpResult {
    match state.stack.pop().unwrap() {
        Value::Pair(start, end) => {
            if start != end {
                let i = seq.cycle.rev % (end - start);
                state.stack.push(state.heap[i]);
            }
            Ok(())
        }
        _ => Err(RuntimeErr::InvalidArgs),
    }
}

/// Reverse a list every other cycle
pub fn palindrome(seq: &mut SeqState, state: &mut InterpState) -> InterpResult {
    if seq.cycle.rev % 2 == 1 {
        return reverse(seq, state);
    }
    Ok(())
}

/// Generate a rhythm using the Hop-and-jump algorithm
///
/// Rhythms that satisfy the rhythmic oddity property. See [1]
///
///   [1]: Simha Arom. African Polyphony and Polyrhythm.
///        Cambridge University Press, Cambridge, England, 1991.
pub fn hopjump(_: &mut SeqState, state: &mut InterpState) -> InterpResult {
    let hopsize: Option<f64> = state.stack.pop().unwrap().into();
    let hopsize = hopsize.unwrap() as usize;
    let pulses: Option<f64> = state.stack.pop().unwrap().into();
    let pulses = pulses.unwrap() as usize;
    let onsets: Option<f64> = state.stack.pop().unwrap().into();
    let onsets = onsets.unwrap() as usize;

    if onsets * hopsize >= pulses {
        return Err(RuntimeErr::InvalidArgs);
    }

    let mut rhythm: Vec<u8> = vec![0; pulses];
    let mut onset = 0;
    let mut pulse = 0;

    loop {
        if onset >= onsets {
            break;
        }
        let value = rhythm[pulse];
        let opposing = pulse + (pulses / 2);
        if value == 0 {
            rhythm[pulse] = 1;
            if opposing < pulses {
                rhythm[opposing] = 2;
            }
            onset += 1;
            pulse = onset * hopsize;
        } else {
            pulse += 1
        }
    }

    let start = state.heap.len();
    for value in &mut rhythm {
        let value = *value;
        if value == 2 {
            state.heap.push(Value::Number(0.0));
        } else {
            state.heap.push(Value::Number(value as f64));
        }
    }

    state.stack.push(Value::Pair(start, state.heap.len()));
    Ok(())
}

/// Define a list of simultanious events
pub fn simul(_: &mut SeqState, state: &mut InterpState) -> InterpResult {
    match state.stack.pop().unwrap() {
        Value::Pair(start, end) => {
            state.stack.push(Value::Tuple(start, end));
            Ok(())
        }
        _ => Err(RuntimeErr::InvalidArgs),
    }
}

/// Build a track by recursively subdividing a list into a series of events
pub fn track(seq: &mut SeqState, state: &mut InterpState) -> InterpResult {
    let num: Option<f64> = state.stack.pop().unwrap().into();
    let num = num.unwrap() as u32;

    let dur: Option<f64> = state.stack.pop().unwrap().into();
    let dur = dur.unwrap();

    let mut track = SeqTrack {
        num: num as usize,
        events: Vec::new(),
        dur: dur,
    };

    let mut visit: Vec<(f64, f64, Value)> = Vec::new();
    visit.push((0.0, dur, state.stack.pop().unwrap()));

    while let Some((onset, dur, val)) = visit.pop() {
        match val {
            Value::Curve(points) => {
                let event = Event {
                    track: num,
                    onset: onset,
                    dur: dur,
                    value: EventValue::Curve(points),
                };
                track.events.push(event);
            }
            Value::Null => (),
            Value::Number(val) => {
                let event = Event {
                    track: num,
                    onset: onset,
                    dur: dur,
                    value: EventValue::Trigger(val),
                };
                track.events.push(event);
            }
            Value::Pair(start, end) => {
                let interval = dur / (end - start) as f64;
                let mut onset = onset;
                for n in start..end {
                    visit.push((onset, interval, state.heap[n]));
                    onset += interval;
                }
            }
            Value::Tuple(start, end) => {
                for n in start..end {
                    visit.push((onset, dur, state.heap[n]));
                }
            }
            _ => return Err(RuntimeErr::InvalidArgs),
        }
    }

    seq.tracks.push(track);
    Ok(())
}

/// Create a bezier curve from a linear ramp
pub fn linear(_: &mut SeqState, state: &mut InterpState) -> InterpResult {
    match state.stack.pop().unwrap() {
        Value::Pair(start, end) => {
            if end - start != 2 {
                return Err(RuntimeErr::InvalidArgs);
            }

            let c0: Option<f64> = state.heap[start].into();
            let c0 = c0.unwrap();
            let c1: Option<f64> = state.heap[start + 1].into();
            let c1 = c1.unwrap();
            let curve = path_to_curve(&[0.0, c0 as f64], &[1.0, c1 as f64]);

            state.stack.push(Value::Curve(curve));

            Ok(())
        }
        _ => Err(RuntimeErr::InvalidArgs),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn repeat_keyword() {
        let mut state = InterpState::new();
        let mut seq = SeqState::new();
        state.stack.push(Value::Number(12.0));
        state.stack.push(Value::Number(3.0));
        repeat(&mut seq, &mut state).unwrap();
        let a: Option<f64> = state.stack.pop().unwrap().into();
        let a = a.unwrap();
        let b: Option<f64> = state.stack.pop().unwrap().into();
        let b = b.unwrap();
        let c: Option<f64> = state.stack.pop().unwrap().into();
        let c = c.unwrap();
        assert_eq!(a, 12.0);
        assert_eq!(b, 12.0);
        assert_eq!(c, 12.0);
        assert_eq!(state.stack.len(), 0);
    }

    #[test]
    fn every_keyword_true() {
        let mut state = InterpState::new();
        let mut seq = SeqState::new();
        seq.cycle.rev = 3;
        state.stack.push(Value::Number(3.14));
        state.stack.push(Value::Number(2.17));
        state.stack.push(Value::Number(3.0));
        every(&mut seq, &mut state).unwrap();
        let a: Option<f64> = state.stack.pop().unwrap().into();
        let a = a.unwrap();
        assert_eq!(a, 3.14);
        assert_eq!(state.stack.len(), 0);
    }

    #[test]
    fn every_keyword_false() {
        let mut state = InterpState::new();
        let mut seq = SeqState::new();
        seq.cycle.rev = 3;
        state.stack.push(Value::Number(3.14));
        state.stack.push(Value::Number(2.17));
        state.stack.push(Value::Number(4.0));
        every(&mut seq, &mut state).unwrap();
        let a: Option<f64> = state.stack.pop().unwrap().into();
        let a = a.unwrap();
        assert_eq!(a, 2.17);
        assert_eq!(state.stack.len(), 0);
    }

    #[test]
    fn reverse_keyword() {
        let mut state = InterpState::new();
        let mut seq = SeqState::new();
        state.heap.push(Value::Number(1.0));
        state.heap.push(Value::Number(2.0));
        state.heap.push(Value::Number(3.0));
        state.stack.push(Value::Pair(0, 3));
        reverse(&mut seq, &mut state).unwrap();
        let a: Option<f64> = state.heap.remove(0).into();
        let a = a.unwrap();
        let b: Option<f64> = state.heap.remove(0).into();
        let b = b.unwrap();
        let c: Option<f64> = state.heap.remove(0).into();
        let c = c.unwrap();
        assert_eq!(a, 3.0);
        assert_eq!(b, 2.0);
        assert_eq!(c, 1.0);
        assert_eq!(state.stack.len(), 1);
    }

    #[test]
    fn rotate_keyword() {
        let mut state = InterpState::new();
        let mut seq = SeqState::new();
        state.heap.push(Value::Number(1.0));
        state.heap.push(Value::Number(2.0));
        state.heap.push(Value::Number(3.0));
        state.stack.push(Value::Pair(0, 3));
        state.stack.push(Value::Number(1.0));
        rotate(&mut seq, &mut state).unwrap();
        let a: Option<f64> = state.heap.remove(0).into();
        let a = a.unwrap();
        let b: Option<f64> = state.heap.remove(0).into();
        let b = b.unwrap();
        let c: Option<f64> = state.heap.remove(0).into();
        let c = c.unwrap();
        assert_eq!(a, 3.0);
        assert_eq!(b, 1.0);
        assert_eq!(c, 2.0);
        assert_eq!(state.stack.len(), 1);
    }

    #[test]
    fn hopjump_keyword() {
        let mut state = InterpState::new();
        let mut seq = SeqState::new();
        state.stack.push(Value::Number(5.0));
        state.stack.push(Value::Number(12.0));
        state.stack.push(Value::Number(2.0));
        hopjump(&mut seq, &mut state).unwrap();
        assert_eq!(state.stack.len(), 1);
    }

    #[test]
    fn test_simultaneous_events() {
        let mut state = InterpState::new();
        let mut seq = SeqState::new();
        state.heap.push(Value::Number(1.0));
        state.heap.push(Value::Number(2.0));
        state.heap.push(Value::Number(3.0));
        state.stack.push(Value::Pair(0, 3));
        simul(&mut seq, &mut state).unwrap();
        state.stack.push(Value::Number(1000.0));
        state.stack.push(Value::Number(0.0));
        track(&mut seq, &mut state).unwrap();

        assert_eq!(seq.tracks[0].events,
                   [Event {
                        track: 0,
                        onset: 0.0,
                        dur: 1000.0,
                        value: EventValue::Trigger(3.0),
                    },
                    Event {
                        track: 0,
                        onset: 0.0,
                        dur: 1000.0,
                        value: EventValue::Trigger(2.0),
                    },
                    Event {
                        track: 0,
                        onset: 0.0,
                        dur: 1000.0,
                        value: EventValue::Trigger(1.0),
                    }]);
    }
}
