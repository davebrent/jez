use vm::interp::InterpState;
use vm::types::{Result, SeqState};


pub fn print(_: &mut SeqState, state: &mut InterpState) -> Result {
    let val = try!(state.last());
    println!("{:?}", val);
    Ok(None)
}

pub fn print_heap(_: &mut SeqState, state: &mut InterpState) -> Result {
    let (start, end) = try!(try!(state.pop()).as_range());
    let slice = try!(state.heap_slice_mut(start, end));
    println!("{:?}", slice);
    Ok(None)
}
