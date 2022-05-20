use rand::Rng;

use crate::vm::interp::{InterpState, Value};
use crate::vm::types::{Result, SeqState};

/// Every cycle, puts the 'next' element of a list on the stack
pub fn cycle(seq: &mut SeqState, state: &mut InterpState) -> Result {
    let (start, end) = (state.pop()?).as_range()?;
    if start != end {
        let i = seq.revision % (end - start);
        let v = state.heap_get(start + i)?;
        state.push(v)?;
    }
    Ok(None)
}

/// Randomly set values to rests in a list
pub fn degrade(seq: &mut SeqState, state: &mut InterpState) -> Result {
    let (start, end) = (state.last()?).as_range()?;
    let lst = state.heap_slice_mut(start, end)?;
    for item in lst {
        if seq.rng.gen() {
            *item = Value::Null;
        }
    }
    Ok(None)
}

/// Put a value on the stack every 'n' cycles
pub fn every(seq: &mut SeqState, state: &mut InterpState) -> Result {
    let freq = state.pop_num()? as usize;
    if freq % seq.revision == 0 {
        state.pop()?;
    } else {
        // Remove the else clause from the stack
        let val = state.pop()?;
        state.pop()?;
        state.push(val)?;
    }
    Ok(None)
}

/// Reverse a list every other cycle
pub fn palindrome(seq: &mut SeqState, state: &mut InterpState) -> Result {
    if seq.revision % 2 == 1 {
        return reverse(seq, state);
    }
    Ok(None)
}

/// Construct a continuous integer sequence from `a` to `b`
pub fn range(_: &mut SeqState, state: &mut InterpState) -> Result {
    let b = state.pop_num()? as usize;
    let a = state.pop_num()? as usize;

    let start = state.heap_len();
    for i in a..b {
        state.heap_push(Value::Number(i as f64));
    }

    let end = state.heap_len();
    state.push(Value::Seq(start, end))?;
    Ok(None)
}

/// Repeat a value 'n' times
pub fn repeat(_: &mut SeqState, state: &mut InterpState) -> Result {
    let times = state.pop_num()? as usize;
    let val = state.pop()?;
    for _ in 0..times {
        state.push(val.clone())?;
    }
    Ok(None)
}

/// Reverse a list, leaving it on the stack
pub fn reverse(_: &mut SeqState, state: &mut InterpState) -> Result {
    let (start, end) = (state.last()?).as_range()?;
    let slice = state.heap_slice_mut(start, end)?;
    slice.reverse();
    Ok(None)
}

/// Rotate a list
pub fn rotate(_: &mut SeqState, state: &mut InterpState) -> Result {
    let amount = state.pop_num()? as usize;
    let (start, end) = (state.last()?).as_range()?;

    let lst = (state.heap_slice_mut(start, end)?).to_vec();
    let len = lst.len();
    let (a, b) = lst.split_at(len - (amount % len));
    let mut out = Vec::new();
    out.extend_from_slice(b);
    out.extend_from_slice(a);
    let slice = state.heap_slice_mut(start, end)?;
    slice.clone_from_slice(&out);
    Ok(None)
}

/// Shuffle a list, leaving it on the stack
pub fn shuffle(seq: &mut SeqState, state: &mut InterpState) -> Result {
    let (start, end) = (state.last()?).as_range()?;
    let slice = state.heap_slice_mut(start, end)?;
    seq.rng.shuffle(slice);
    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_range() {
        let mut state = InterpState::new();
        let mut seq = SeqState::new();
        state.call(0, 0, 1).unwrap();
        state.push(Value::Number(2.0)).unwrap();
        state.push(Value::Number(6.0)).unwrap();
        range(&mut seq, &mut state).unwrap();
        let out = state.heap_slice_mut(0, 4).unwrap();
        assert_eq!(
            out,
            &[
                Value::Number(2.0),
                Value::Number(3.0),
                Value::Number(4.0),
                Value::Number(5.0),
            ]
        );
    }

    #[test]
    fn repeat_keyword() {
        let mut state = InterpState::new();
        let mut seq = SeqState::new();
        state.call(0, 0, 1).unwrap();
        state.push(Value::Number(12.0)).unwrap();
        state.push(Value::Number(3.0)).unwrap();
        repeat(&mut seq, &mut state).unwrap();
        assert_eq!(state.pop_num().unwrap(), 12.0);
        assert_eq!(state.pop_num().unwrap(), 12.0);
        assert_eq!(state.pop_num().unwrap(), 12.0);
        assert_eq!(state.pop().is_err(), true);
    }

    #[test]
    fn every_keyword_true() {
        let mut state = InterpState::new();
        let mut seq = SeqState::new();
        seq.revision = 3;
        state.call(0, 0, 1).unwrap();
        state.push(Value::Number(3.14)).unwrap();
        state.push(Value::Number(2.17)).unwrap();
        state.push(Value::Number(3.0)).unwrap();
        every(&mut seq, &mut state).unwrap();
        assert_eq!(state.pop_num().unwrap(), 3.14);
        assert_eq!(state.pop().is_err(), true);
    }

    #[test]
    fn every_keyword_false() {
        let mut state = InterpState::new();
        let mut seq = SeqState::new();
        seq.revision = 3;
        state.call(0, 0, 1).unwrap();
        state.push(Value::Number(3.14)).unwrap();
        state.push(Value::Number(2.17)).unwrap();
        state.push(Value::Number(4.0)).unwrap();
        every(&mut seq, &mut state).unwrap();
        assert_eq!(state.pop_num().unwrap(), 2.17);
        assert_eq!(state.pop().is_err(), true);
    }

    #[test]
    fn reverse_keyword() {
        let mut state = InterpState::new();
        let mut seq = SeqState::new();
        state.call(0, 0, 1).unwrap();
        state.heap_push(Value::Number(1.0));
        state.heap_push(Value::Number(2.0));
        state.heap_push(Value::Number(3.0));
        state.push(Value::Seq(0, 3)).unwrap();
        reverse(&mut seq, &mut state).unwrap();
        let out = state.heap_slice_mut(0, 3).unwrap();
        assert_eq!(out[0].as_num().unwrap(), 3.0);
        assert_eq!(out[1].as_num().unwrap(), 2.0);
        assert_eq!(out[2].as_num().unwrap(), 1.0);
    }

    #[test]
    fn rotate_keyword() {
        let mut state = InterpState::new();
        let mut seq = SeqState::new();
        state.call(0, 0, 1).unwrap();
        state.heap_push(Value::Number(1.0));
        state.heap_push(Value::Number(2.0));
        state.heap_push(Value::Number(3.0));
        state.push(Value::Seq(0, 3)).unwrap();
        state.push(Value::Number(1.0)).unwrap();
        rotate(&mut seq, &mut state).unwrap();
        let out = state.heap_slice_mut(0, 3).unwrap();
        assert_eq!(out[0].as_num().unwrap(), 3.0);
        assert_eq!(out[1].as_num().unwrap(), 1.0);
        assert_eq!(out[2].as_num().unwrap(), 2.0);
    }
}
