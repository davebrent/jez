use unit::{EventValue, InterpState, InterpResult, Value};

use super::state::{MidiState, MidiMessage};


/// Put the value of the current event on the stack
pub fn event_value(ms: &mut MidiState, is: &mut InterpState) -> InterpResult {
    match ms.event.value {
        EventValue::Curve(curve) => {
            is.stack.push(Value::Curve(curve));
        }
        EventValue::Trigger(val) => {
            is.stack.push(Value::Number(val));
        }
    }
    Ok(())
}

/// Put the duration of the current event on the stack
pub fn event_duration(ms: &mut MidiState,
                      is: &mut InterpState)
                      -> InterpResult {
    is.stack.push(Value::Number(ms.event.dur));
    Ok(())
}

/// Dispatch a note event
pub fn noteout(ms: &mut MidiState, is: &mut InterpState) -> InterpResult {
    let channel: Option<f64> = is.stack.pop().unwrap().into();
    let duration: Option<f64> = is.stack.pop().unwrap().into();
    let velocity: Option<f64> = is.stack.pop().unwrap().into();
    let pitch: Option<f64> = is.stack.pop().unwrap().into();
    ms.message = MidiMessage::Note {
        channel: channel.unwrap() as u8,
        pitch: pitch.unwrap() as u8,
        velocity: velocity.unwrap() as u8,
        duration: duration.unwrap(),
    };
    Ok(())
}

/// Dispatch a note event
pub fn ctrlout(ms: &mut MidiState, is: &mut InterpState) -> InterpResult {
    let channel: Option<f64> = is.stack.pop().unwrap().into();
    let ctrl: Option<f64> = is.stack.pop().unwrap().into();
    ms.message = MidiMessage::Ctrl {
        channel: channel.unwrap() as u8,
        ctrl: ctrl.unwrap() as u8,
    };
    Ok(())
}
