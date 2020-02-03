use crate::vm::interp::{InterpState, Value};
use crate::vm::types::{Result, SeqState};

pub fn add(_: &mut SeqState, state: &mut InterpState) -> Result {
    let rhs = state.pop_num()?;
    let lhs = state.pop_num()?;
    state.push(Value::Number(lhs + rhs))?;
    Ok(None)
}

pub fn subtract(_: &mut SeqState, state: &mut InterpState) -> Result {
    let rhs = state.pop_num()?;
    let lhs = state.pop_num()?;
    state.push(Value::Number(lhs - rhs))?;
    Ok(None)
}

pub fn multiply(_: &mut SeqState, state: &mut InterpState) -> Result {
    let rhs = state.pop_num()?;
    let lhs = state.pop_num()?;
    state.push(Value::Number(lhs * rhs))?;
    Ok(None)
}

pub fn divide(_: &mut SeqState, state: &mut InterpState) -> Result {
    let rhs = state.pop_num()?;
    let lhs = state.pop_num()?;
    state.push(Value::Number(lhs / rhs))?;
    Ok(None)
}

pub fn modulo(_: &mut SeqState, state: &mut InterpState) -> Result {
    let rhs = state.pop_num()?;
    let lhs = state.pop_num()?;
    state.push(Value::Number(lhs % rhs))?;
    Ok(None)
}
