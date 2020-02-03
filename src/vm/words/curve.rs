use crate::vm::interp::{InterpState, Value};
use crate::vm::math::path_to_curve;
use crate::vm::types::{Result, SeqState};

/// Create a bezier curve from a linear ramp
pub fn linear(_: &mut SeqState, state: &mut InterpState) -> Result {
    let (start, end) = r#try!(r#try!(state.pop()).as_range());
    if end - start != 2 {
        return Err(error!(InvalidArgs));
    }

    let c0 = r#try!(r#try!(state.heap_get(start)).as_num());
    let c1 = r#try!(r#try!(state.heap_get(start + 1)).as_num());
    let curve = path_to_curve(&[0.0, c0 as f64], &[1.0, c1 as f64]);
    r#try!(state.push(Value::Curve(curve)));
    Ok(None)
}
