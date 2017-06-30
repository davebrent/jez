use err::RuntimeErr;
use interp::{InterpState, InterpResult, Value};
use unit::EventValue;

use super::state::{MidiState, MidiMessage};


/// Put the value of the current event on the stack
pub fn event_value(ms: &mut MidiState, is: &mut InterpState) -> InterpResult {
    match ms.event.value {
        EventValue::Curve(curve) => {
            try!(is.push(Value::Curve(curve)));
        }
        EventValue::Trigger(val) => {
            try!(is.push(Value::Number(val)));
        }
    }
    Ok(None)
}

/// Put the duration of the current event on the stack
pub fn event_duration(ms: &mut MidiState,
                      is: &mut InterpState)
                      -> InterpResult {
    try!(is.push(Value::Number(ms.event.dur)));
    Ok(None)
}

/// Put the track of the current event on the stack
pub fn event_track(ms: &mut MidiState, is: &mut InterpState) -> InterpResult {
    try!(is.push(Value::Number(ms.event.track as f64)));
    Ok(None)
}

/// Dispatch a note event
pub fn noteout(ms: &mut MidiState, is: &mut InterpState) -> InterpResult {
    let channel = try!(is.pop_num()) as u8;
    let duration = try!(is.pop_num());
    let velocity = try!(is.pop_num()) as u8;
    let pitch = try!(is.pop_num()) as u8;

    if channel >= 16 || pitch >= 128 || velocity >= 128 {
        return Err(RuntimeErr::InvalidArgs);
    }

    ms.message = MidiMessage::Note {
        channel: channel,
        pitch: pitch,
        velocity: velocity,
        duration: duration,
    };
    Ok(None)
}

/// Dispatch a controller event
pub fn ctrlout(ms: &mut MidiState, is: &mut InterpState) -> InterpResult {
    let channel = try!(is.pop_num()) as u8;
    let ctrl = try!(is.pop_num()) as u8;

    if channel >= 16 || ctrl >= 120 {
        return Err(RuntimeErr::InvalidArgs);
    }

    ms.message = MidiMessage::Ctrl {
        channel: channel,
        ctrl: ctrl,
    };
    Ok(None)
}
