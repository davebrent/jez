use unit::{InterpState, InterpResult, Value};

use super::state::MidiState;


/// Puts the value of the current event onto the stack
pub fn event_value(ms: &mut MidiState, is: &mut InterpState) -> InterpResult {
    is.stack.push(Value::Number(ms.event.value));
    Ok(())
}

/// Puts the duration of the current event onto the stack
pub fn event_duration(ms: &mut MidiState,
                      is: &mut InterpState)
                      -> InterpResult {
    is.stack.push(Value::Number(ms.event.duration));
    Ok(())
}

/// Trigger a note off event
pub fn makenote(_: &mut MidiState, _: &mut InterpState) -> InterpResult {
    Ok(())
}

/// Dispatch a note event
pub fn noteout(_: &mut MidiState, _: &mut InterpState) -> InterpResult {
    Ok(())
}
