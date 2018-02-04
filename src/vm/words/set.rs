use std::collections::BTreeSet;
use std::result;

use err::RuntimeErr;
use vm::interp::{InterpState, Value};
use vm::types::{Result, SeqState};


/// Apply a residual class to an integer sequence
pub fn sieve(_: &mut SeqState, state: &mut InterpState) -> Result {
    let (modulus, shift) = try!(state.pop_pair());
    let (start, end) = try!(state.pop_pair());

    let next_start = state.heap_len();
    for ptr in start..end {
        let val = try!(try!(state.heap_get(ptr)).as_num()) as usize;
        if val % modulus == shift {
            state.heap_push(Value::Number(val as f64));
        }
    }

    let next_end = state.heap_len();
    try!(state.push(Value::Pair(next_start, next_end)));
    Ok(None)
}

pub fn _pop_set(state: &mut InterpState)
                -> result::Result<BTreeSet<usize>, RuntimeErr> {
    let (start, end) = try!(state.pop_pair());
    let mut output = BTreeSet::new();

    for ptr in start..end {
        let val = try!(try!(state.heap_get(ptr)).as_num()) as usize;
        output.insert(val);
    }

    Ok(output)
}

/// Perform the intersection ('or') of two lists
pub fn intersection(_: &mut SeqState, state: &mut InterpState) -> Result {
    let a = try!(_pop_set(state));
    let b = try!(_pop_set(state));
    let vals: Vec<usize> = a.intersection(&b).cloned().collect();

    let start = state.heap_len();
    for val in vals {
        state.heap_push(Value::Number(val as f64));
    }

    let end = state.heap_len();
    try!(state.push(Value::Pair(start, end)));
    Ok(None)
}

/// Perform the union ('and') of two lists
pub fn union(_: &mut SeqState, state: &mut InterpState) -> Result {
    let a = try!(_pop_set(state));
    let b = try!(_pop_set(state));
    let vals: Vec<usize> = a.union(&b).cloned().collect();

    let start = state.heap_len();
    for val in vals {
        state.heap_push(Value::Number(val as f64));
    }

    let end = state.heap_len();
    try!(state.push(Value::Pair(start, end)));
    Ok(None)
}

/// Perform the symmetric difference ('xor') between two lists
pub fn symmetric_difference(_: &mut SeqState,
                            state: &mut InterpState)
                            -> Result {
    let a = try!(_pop_set(state));
    let b = try!(_pop_set(state));
    let vals: Vec<usize> = a.symmetric_difference(&b).cloned().collect();

    let start = state.heap_len();
    for val in vals {
        state.heap_push(Value::Number(val as f64));
    }

    let end = state.heap_len();
    try!(state.push(Value::Pair(start, end)));
    Ok(None)
}
