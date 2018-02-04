use err::RuntimeErr;
use vm::interp::{InterpState, Value};
use vm::math::path_to_curve;
use vm::types::{Result, SeqState};


/// Create a bezier curve from a linear ramp
pub fn linear(_: &mut SeqState, state: &mut InterpState) -> Result {
    let (start, end) = try!(state.pop_pair());
    if end - start != 2 {
        return Err(RuntimeErr::InvalidArgs);
    }

    let c0 = try!(try!(state.heap_get(start)).as_num());
    let c1 = try!(try!(state.heap_get(start + 1)).as_num());
    let curve = path_to_curve(&[0.0, c0 as f64], &[1.0, c1 as f64]);
    try!(state.push(Value::Curve(curve)));
    Ok(None)
}
