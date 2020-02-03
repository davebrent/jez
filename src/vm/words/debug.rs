use crate::vm::interp::InterpState;
use crate::vm::types::{Result, SeqState};

pub fn print(_: &mut SeqState, state: &mut InterpState) -> Result {
    let val = r#try!(state.last());
    println!("{:?}", val);
    Ok(None)
}

pub fn print_heap(_: &mut SeqState, state: &mut InterpState) -> Result {
    let (start, end) = r#try!(r#try!(state.pop()).as_range());
    let slice = r#try!(state.heap_slice_mut(start, end));
    println!("{:?}", slice);
    Ok(None)
}
