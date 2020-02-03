use crate::vm::interp::InterpState;
use crate::vm::types::{Result, SeqState};

pub fn print(_: &mut SeqState, state: &mut InterpState) -> Result {
    let val = state.last()?;
    println!("{:?}", val);
    Ok(None)
}

pub fn print_heap(_: &mut SeqState, state: &mut InterpState) -> Result {
    let (start, end) = (state.pop()?).as_range()?;
    let slice = state.heap_slice_mut(start, end)?;
    println!("{:?}", slice);
    Ok(None)
}
