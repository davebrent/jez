use rand::{Rng, StdRng};

use unit::{InterpState, RuntimeErr, InterpResult, Value};

use super::seq::{Event, SeqState};


/// Repeat a value 'n' times
pub fn repeat(_: &mut SeqState, state: &mut InterpState) -> InterpResult {
    let times: Option<f32> = state.stack.pop().unwrap().into();
    let times = times.unwrap() as u32;
    let val = state.stack.pop().unwrap();
    for _ in 0..times {
        state.stack.push(val.clone());
    }
    Ok(())
}

/// Put a value on the stack every 'n' cycles
pub fn every(seq: &mut SeqState, state: &mut InterpState) -> InterpResult {
    let freq: Option<f32> = state.stack.pop().unwrap().into();
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
            &state.heap[start..end].reverse();
            Ok(())
        }
        _ => Err(RuntimeErr::WrongType),
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
        _ => Err(RuntimeErr::WrongType),
    }
}

/// Rotate a list
pub fn rotate(_: &mut SeqState, state: &mut InterpState) -> InterpResult {
    let amount: Option<f32> = state.stack.pop().unwrap().into();
    let amount = amount.unwrap() as usize;
    match *state.stack.last().unwrap() {
        Value::Pair(start, end) => {
            let lst = &state.heap[start..end].to_vec();
            let (a, b) = lst.split_at(lst.len() - amount);
            let mut out = Vec::new();
            out.extend_from_slice(b);
            out.extend_from_slice(a);
            &state.heap[start..end].copy_from_slice(&out);
            Ok(())
        }
        _ => Err(RuntimeErr::WrongType),
    }
}

/// Randomly set values to rests in a list
pub fn degrade(_: &mut SeqState, state: &mut InterpState) -> InterpResult {
    let mut rng = StdRng::new().unwrap();
    match *state.stack.last().unwrap() {
        Value::Pair(start, end) => {
            let lst = &mut state.heap[start..end];
            for i in 0..lst.len() {
                if rng.gen() {
                    lst[i] = Value::Null;
                }
            }
            Ok(())
        }
        _ => Err(RuntimeErr::WrongType),
    }
}

/// Every cycle, puts the 'next' element of a list on the stack
pub fn cycle(seq: &mut SeqState, state: &mut InterpState) -> InterpResult {
    match state.stack.pop().unwrap() {
        Value::Pair(start, end) => {
            if start != end {
                let i = seq.cycle.rev % (end - start);
                state.stack.push(state.heap.get(i).unwrap().clone());
            }
            Ok(())
        }
        _ => Err(RuntimeErr::WrongType),
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
    let hopsize: Option<f32> = state.stack.pop().unwrap().into();
    let hopsize = hopsize.unwrap() as usize;
    let pulses: Option<f32> = state.stack.pop().unwrap().into();
    let pulses = pulses.unwrap() as usize;
    let onsets: Option<f32> = state.stack.pop().unwrap().into();
    let onsets = onsets.unwrap() as usize;

    if onsets * hopsize >= pulses {
        return Err(RuntimeErr::InvalidArguments);
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
            state.heap.push(Value::Number(0f32));
        } else {
            state.heap.push(Value::Number(value as f32));
        }
    }

    state.stack.push(Value::Pair(start, state.heap.len()));
    Ok(())
}

/// Build a track by recursively subdividing a list into a series of events
pub fn track(seq: &mut SeqState, state: &mut InterpState) -> InterpResult {
    let no: Option<f32> = state.stack.pop().unwrap().into();
    let no = no.unwrap() as u32;

    let dur: Option<f32> = state.stack.pop().unwrap().into();
    let dur = dur.unwrap();

    seq.cycle.dur = dur;

    let mut visit: Vec<(f32, f32, Value)> = Vec::new();
    visit.push((0f32, dur, state.stack.pop().unwrap()));

    while let Some((onset, dur, val)) = visit.pop() {
        match val {
            Value::Null => (),
            Value::Number(val) => {
                seq.events.push(Event::new(no, onset, dur, val));
            }
            Value::Pair(start, end) => {
                let interval = dur / (end - start) as f32;
                let mut onset = onset;
                for n in start..end {
                    visit.push((onset, interval, *state.heap.get(n).unwrap()));
                    onset += interval;
                }
            }
            _ => return Err(RuntimeErr::WrongType),
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn repeat_keyword() {
        let mut state = InterpState::new();
        let mut seq = SeqState::new();
        state.stack.push(Value::Number(12f32));
        state.stack.push(Value::Number(3f32));
        repeat(&mut seq, &mut state).unwrap();
        let a: Option<f32> = state.stack.pop().unwrap().into();
        let a = a.unwrap();
        let b: Option<f32> = state.stack.pop().unwrap().into();
        let b = b.unwrap();
        let c: Option<f32> = state.stack.pop().unwrap().into();
        let c = c.unwrap();
        assert_eq!(a, 12f32);
        assert_eq!(b, 12f32);
        assert_eq!(c, 12f32);
        assert_eq!(state.stack.len(), 0);
    }

    #[test]
    fn every_keyword_true() {
        let mut state = InterpState::new();
        let mut seq = SeqState::new();
        seq.cycle.rev = 3;
        state.stack.push(Value::Number(3.14));
        state.stack.push(Value::Number(2.17));
        state.stack.push(Value::Number(3f32));
        every(&mut seq, &mut state).unwrap();
        let a: Option<f32> = state.stack.pop().unwrap().into();
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
        state.stack.push(Value::Number(4f32));
        every(&mut seq, &mut state).unwrap();
        let a: Option<f32> = state.stack.pop().unwrap().into();
        let a = a.unwrap();
        assert_eq!(a, 2.17);
        assert_eq!(state.stack.len(), 0);
    }

    #[test]
    fn reverse_keyword() {
        let mut state = InterpState::new();
        let mut seq = SeqState::new();
        state.heap.push(Value::Number(1f32));
        state.heap.push(Value::Number(2f32));
        state.heap.push(Value::Number(3f32));
        state.stack.push(Value::Pair(0, 3));
        reverse(&mut seq, &mut state).unwrap();
        let a: Option<f32> = state.heap.remove(0).into();
        let a = a.unwrap();
        let b: Option<f32> = state.heap.remove(0).into();
        let b = b.unwrap();
        let c: Option<f32> = state.heap.remove(0).into();
        let c = c.unwrap();
        assert_eq!(a, 3f32);
        assert_eq!(b, 2f32);
        assert_eq!(c, 1f32);
        assert_eq!(state.stack.len(), 1);
    }

    #[test]
    fn rotate_keyword() {
        let mut state = InterpState::new();
        let mut seq = SeqState::new();
        state.heap.push(Value::Number(1f32));
        state.heap.push(Value::Number(2f32));
        state.heap.push(Value::Number(3f32));
        state.stack.push(Value::Pair(0, 3));
        state.stack.push(Value::Number(1f32));
        rotate(&mut seq, &mut state).unwrap();
        let a: Option<f32> = state.heap.remove(0).into();
        let a = a.unwrap();
        let b: Option<f32> = state.heap.remove(0).into();
        let b = b.unwrap();
        let c: Option<f32> = state.heap.remove(0).into();
        let c = c.unwrap();
        assert_eq!(a, 3f32);
        assert_eq!(b, 1f32);
        assert_eq!(c, 2f32);
        assert_eq!(state.stack.len(), 1);
    }

    #[test]
    fn hopjump_keyword() {
        let mut state = InterpState::new();
        let mut seq = SeqState::new();
        state.stack.push(Value::Number(5f32));
        state.stack.push(Value::Number(12f32));
        state.stack.push(Value::Number(2f32));
        hopjump(&mut seq, &mut state).unwrap();
        assert_eq!(state.stack.len(), 1);
    }
}
