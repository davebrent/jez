use vm::interp::{InterpState, Value};
use vm::types::{Result, SeqState};


pub fn pair(_: &mut SeqState, state: &mut InterpState) -> Result {
    let b = try!(state.pop_num()) as usize;
    let a = try!(state.pop_num()) as usize;
    try!(state.push(Value::Pair(a, b)));
    Ok(None)
}

pub fn drop(_: &mut SeqState, state: &mut InterpState) -> Result {
    try!(state.pop());
    Ok(None)
}

pub fn duplicate(_: &mut SeqState, state: &mut InterpState) -> Result {
    let val = try!(state.last());
    try!(state.push(val));
    Ok(None)
}

pub fn swap(_: &mut SeqState, state: &mut InterpState) -> Result {
    let a = try!(state.pop());
    let b = try!(state.pop());
    try!(state.push(a));
    try!(state.push(b));
    Ok(None)
}
