use err::RuntimeErr;
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

/// Put the track of the current event on the stack
pub fn event_track(ms: &mut MidiState, is: &mut InterpState) -> InterpResult {
    is.stack.push(Value::Number(ms.event.track as f64));
    Ok(())
}

/// Dispatch a note event
pub fn noteout(ms: &mut MidiState, is: &mut InterpState) -> InterpResult {
    let channel: Option<f64> = is.stack.pop().unwrap().into();
    let channel = channel.unwrap() as u8;
    let duration: Option<f64> = is.stack.pop().unwrap().into();
    let velocity: Option<f64> = is.stack.pop().unwrap().into();
    let velocity = velocity.unwrap() as u8;
    let pitch: Option<f64> = is.stack.pop().unwrap().into();
    let pitch = pitch.unwrap() as u8;

    if channel >= 16 || pitch >= 128 || velocity >= 128 {
        return Err(RuntimeErr::InvalidArgs);
    }

    ms.message = MidiMessage::Note {
        channel: channel,
        pitch: pitch,
        velocity: velocity,
        duration: duration.unwrap(),
    };
    Ok(())
}

/// Dispatch a controller event
pub fn ctrlout(ms: &mut MidiState, is: &mut InterpState) -> InterpResult {
    let channel: Option<f64> = is.stack.pop().unwrap().into();
    let channel = channel.unwrap() as u8;
    let ctrl: Option<f64> = is.stack.pop().unwrap().into();
    let ctrl = ctrl.unwrap() as u8;

    if channel >= 16 || ctrl >= 120 {
        return Err(RuntimeErr::InvalidArgs);
    }

    ms.message = MidiMessage::Ctrl {
        channel: channel,
        ctrl: ctrl,
    };
    Ok(())
}
