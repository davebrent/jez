use crate::vm::interp::InterpState;
use crate::vm::types::{Result, SeqState};

pub fn drop(_: &mut SeqState, state: &mut InterpState) -> Result {
    state.pop()?;
    Ok(None)
}

pub fn duplicate(_: &mut SeqState, state: &mut InterpState) -> Result {
    let val = state.last()?;
    state.push(val)?;
    Ok(None)
}

pub fn swap(_: &mut SeqState, state: &mut InterpState) -> Result {
    let a = state.pop()?;
    let b = state.pop()?;
    state.push(a)?;
    state.push(b)?;
    Ok(None)
}
