use crate::vm::interp::InterpState;
use crate::vm::types::{Result, SeqState};

pub fn drop(_: &mut SeqState, state: &mut InterpState) -> Result {
    r#try!(state.pop());
    Ok(None)
}

pub fn duplicate(_: &mut SeqState, state: &mut InterpState) -> Result {
    let val = r#try!(state.last());
    r#try!(state.push(val));
    Ok(None)
}

pub fn swap(_: &mut SeqState, state: &mut InterpState) -> Result {
    let a = r#try!(state.pop());
    let b = r#try!(state.pop());
    r#try!(state.push(a));
    r#try!(state.push(b));
    Ok(None)
}
