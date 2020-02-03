use crate::vm::interp::{InterpState, Value};
use crate::vm::types::{Result, SeqState};

/// Puts the current cycle revision onto the stack
pub fn revision(seq: &mut SeqState, state: &mut InterpState) -> Result {
    r#try!(state.push(Value::Number(seq.revision as f64)));
    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rev() {
        let mut state = InterpState::new();
        let mut seq = SeqState::new();
        seq.revision = 99;
        state.call(0, 0, 1).unwrap();
        revision(&mut seq, &mut state).unwrap();
        assert_eq!(state.pop_num().unwrap(), 99.0);
    }
}
