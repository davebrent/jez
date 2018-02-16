use std::iter;

use err::RuntimeErr;
use vm::interp::{InterpState, Value};
use vm::types::{Result, SeqState};

/// Generate a rhythm using the Hop-and-jump algorithm
///
/// Rhythms that satisfy the rhythmic oddity property. See [1]
///
///   [1]: Simha Arom. African Polyphony and Polyrhythm.
///        Cambridge University Press, Cambridge, England, 1991.
pub fn hop_jump(_: &mut SeqState, state: &mut InterpState) -> Result {
    let hopsize = try!(state.pop_num()) as usize;
    let pulses = try!(state.pop_num()) as usize;
    let onsets = try!(state.pop_num()) as usize;

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

    let start = state.heap_len();
    for value in &mut rhythm {
        let value = *value;
        if value == 2 {
            state.heap_push(Value::Number(0.0));
        } else {
            state.heap_push(Value::Number(f64::from(value)));
        }
    }

    let len = state.heap_len();
    try!(state.push(Value::Seq(start, len)));
    Ok(None)
}

/// Return a new list containing the difference between consecutive elements
pub fn inter_onset(_: &mut SeqState, state: &mut InterpState) -> Result {
    let (start, end) = try!(try!(state.pop()).as_range());
    let count = end - start;
    let heap_start = state.heap_len();

    if count == 1 {
        state.heap_push(Value::Number(0.0));
    } else if count > 1 {
        for i in start..end - 1 {
            let curr = try!(try!(state.heap_get(i)).as_num());
            let next = try!(try!(state.heap_get(i + 1)).as_num());
            state.heap_push(Value::Number(next - curr as f64));
        }
    }

    let end = state.heap_len();
    try!(state.push(Value::Seq(heap_start, end)));
    Ok(None)
}

/// Return a binary onset representation of a list
pub fn onsets(_: &mut SeqState, state: &mut InterpState) -> Result {
    let (start, end) = try!(try!(state.pop()).as_range());
    let b = try!(state.pop_num()) as usize;
    let a = try!(state.pop_num()) as usize;

    let heap_start = state.heap_len();

    if end - start != 0 {
        let mut out = iter::repeat(0).take(b - a).collect::<Vec<_>>();
        for i in start..end {
            let val = try!(try!(state.heap_get(i)).as_num()) as usize;
            if a <= val && val < b {
                out[val - a] = 1;
            }
        }
        for val in out {
            state.heap_push(Value::Number(f64::from(val)));
        }
    }

    let heap_end = state.heap_len();
    try!(state.push(Value::Seq(heap_start, heap_end)));
    Ok(None)
}
