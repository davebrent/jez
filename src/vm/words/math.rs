use crate::vm::interp::{InterpState, Value};
use crate::vm::types::{Result, SeqState};

pub fn add(_: &mut SeqState, state: &mut InterpState) -> Result {
    let rhs = r#try!(state.pop_num());
    let lhs = r#try!(state.pop_num());
    r#try!(state.push(Value::Number(lhs + rhs)));
    Ok(None)
}

pub fn subtract(_: &mut SeqState, state: &mut InterpState) -> Result {
    let rhs = r#try!(state.pop_num());
    let lhs = r#try!(state.pop_num());
    r#try!(state.push(Value::Number(lhs - rhs)));
    Ok(None)
}

pub fn multiply(_: &mut SeqState, state: &mut InterpState) -> Result {
    let rhs = r#try!(state.pop_num());
    let lhs = r#try!(state.pop_num());
    r#try!(state.push(Value::Number(lhs * rhs)));
    Ok(None)
}

pub fn divide(_: &mut SeqState, state: &mut InterpState) -> Result {
    let rhs = r#try!(state.pop_num());
    let lhs = r#try!(state.pop_num());
    r#try!(state.push(Value::Number(lhs / rhs)));
    Ok(None)
}

pub fn modulo(_: &mut SeqState, state: &mut InterpState) -> Result {
    let rhs = r#try!(state.pop_num());
    let lhs = r#try!(state.pop_num());
    r#try!(state.push(Value::Number(lhs % rhs)));
    Ok(None)
}
