use byteorder::{LittleEndian, WriteBytesExt};
use rand::{Rng, SeedableRng};

use err::RuntimeErr;
use vm::interp::{InterpState, Value};
use vm::types::{Result, SeqState};

/// Push a random integer, within a range, onto the stack
pub fn rand_range(seq: &mut SeqState, state: &mut InterpState) -> Result {
    let max = try!(state.pop_num()) as i64;
    let min = try!(state.pop_num()) as i64;
    let val = seq.rng.gen_range(min, max);
    try!(state.push(Value::Number(val as f64)));
    Ok(None)
}

/// Seed the random number generator
pub fn rand_seed(seq: &mut SeqState, state: &mut InterpState) -> Result {
    let seed = try!(state.pop_num()) as i64;
    let mut wtr = vec![];
    if wtr.write_i64::<LittleEndian>(seed).is_err() {
        return Err(RuntimeErr::InvalidArgs(None));
    }
    let seed: Vec<usize> = wtr.iter().map(|n| *n as usize).collect();
    seq.rng.reseed(seed.as_slice());
    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn random_keywords() {
        let mut state = InterpState::new();
        let mut seq = SeqState::new();
        state.call(0, 0, 1).unwrap();
        state.push(Value::Number(3.0)).unwrap();
        rand_seed(&mut seq, &mut state).unwrap();
        state.push(Value::Number(0.0)).unwrap();
        state.push(Value::Number(100.0)).unwrap();
        rand_range(&mut seq, &mut state).unwrap();
        assert_eq!(state.pop_num().unwrap(), 31.0);
    }
}
