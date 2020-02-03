use crate::vm::interp::{InterpState, Value};
use crate::vm::types::{Result, SeqState};

/// Encode a number into a binary list
pub fn bin_list(_: &mut SeqState, state: &mut InterpState) -> Result {
    let num = r#try!(state.pop_num()) as i64;
    let n = r#try!(state.pop_num()) as i64;

    let start = state.heap_len();
    for i in 0..n {
        let val = if num & (1 << i) > 0 {
            Value::Number(1.0)
        } else {
            Value::Null
        };
        state.heap_push(val);
    }

    let len = state.heap_len();
    r#try!(state.push(Value::Seq(start, len)));
    Ok(None)
}

/// Gray code number encoding
pub fn gray_code(_: &mut SeqState, state: &mut InterpState) -> Result {
    let num = r#try!(state.pop_num()) as i64;
    let num = (num >> 1) ^ num;
    r#try!(state.push(Value::Number(num as f64)));
    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_binlist() {
        let mut state = InterpState::new();
        let mut seq = SeqState::new();
        state.call(0, 0, 1).unwrap();
        state.push(Value::Number(5.0)).unwrap();
        state.push(Value::Number(12.0)).unwrap();
        bin_list(&mut seq, &mut state).unwrap();
        assert_eq!(state.heap_len(), 5);
        let out = state.heap_slice_mut(0, 5).unwrap();
        assert_eq!(
            out,
            &[
                Value::Null,
                Value::Null,
                Value::Number(1.0),
                Value::Number(1.0),
                Value::Null,
            ]
        );
    }

    #[test]
    fn test_graycode() {
        let mut state = InterpState::new();
        let mut seq = SeqState::new();
        state.call(0, 0, 1).unwrap();
        state.push(Value::Number(12.0)).unwrap();
        gray_code(&mut seq, &mut state).unwrap();
        assert_eq!(state.pop_num().unwrap() as i64, 10);
    }
}
