use rand::{Rng, StdRng};
use std::rc::Rc;

use err::RuntimeErr;
use interp::{InterpState, InterpResult, Value};
use lang::hash_str;
use math::path_to_curve;

use super::interp::ExtState;
use super::msgs::{Event, Destination, EventValue};
use super::synths::WaveTable;

/// Repeat a value 'n' times
pub fn repeat(_: &mut ExtState, state: &mut InterpState) -> InterpResult {
    let times = try!(state.pop_num()) as usize;
    let val = try!(state.pop());
    for _ in 0..times {
        try!(state.push(val));
    }
    Ok(None)
}

/// Put a value on the stack every 'n' cycles
pub fn every(seq: &mut ExtState, state: &mut InterpState) -> InterpResult {
    let freq = try!(state.pop_num()) as usize;
    if freq % seq.revision == 0 {
        try!(state.pop());
    } else {
        // Remove the else clause from the stack
        let val = try!(state.pop());
        try!(state.pop());
        try!(state.push(val));
    }
    Ok(None)
}

/// Reverse a list, leaving it on the stack
pub fn reverse(_: &mut ExtState, state: &mut InterpState) -> InterpResult {
    let (start, end) = try!(state.last_pair());
    let slice = try!(state.heap_slice_mut(start, end));
    slice.reverse();
    Ok(None)
}

/// Shuffle a list, leaving it on the stack
pub fn shuffle(_: &mut ExtState, state: &mut InterpState) -> InterpResult {
    let (start, end) = try!(state.last_pair());
    let mut rng = StdRng::new().unwrap();
    let slice = try!(state.heap_slice_mut(start, end));
    rng.shuffle(slice);
    Ok(None)
}

/// Rotate a list
pub fn rotate(_: &mut ExtState, state: &mut InterpState) -> InterpResult {
    let amount = try!(state.pop_num()) as usize;
    let (start, end) = try!(state.last_pair());

    let lst = try!(state.heap_slice_mut(start, end)).to_vec();
    let (a, b) = lst.split_at(lst.len() - amount);
    let mut out = Vec::new();
    out.extend_from_slice(b);
    out.extend_from_slice(a);
    let slice = try!(state.heap_slice_mut(start, end));
    slice.copy_from_slice(&out);
    Ok(None)
}

/// Randomly set values to rests in a list
pub fn degrade(_: &mut ExtState, state: &mut InterpState) -> InterpResult {
    let mut rng = StdRng::new().unwrap();
    let (start, end) = try!(state.last_pair());
    let lst = try!(state.heap_slice_mut(start, end));
    for item in lst {
        if rng.gen() {
            *item = Value::Null;
        }
    }
    Ok(None)
}

/// Every cycle, puts the 'next' element of a list on the stack
pub fn cycle(seq: &mut ExtState, state: &mut InterpState) -> InterpResult {
    let (start, end) = try!(state.pop_pair());
    if start != end {
        let i = seq.revision % (end - start);
        let v = try!(state.heap_get(i));
        try!(state.push(v));
    }
    Ok(None)
}

/// Reverse a list every other cycle
pub fn palindrome(seq: &mut ExtState, state: &mut InterpState) -> InterpResult {
    if seq.revision % 2 == 1 {
        return reverse(seq, state);
    }
    Ok(None)
}

/// Generate a rhythm using the Hop-and-jump algorithm
///
/// Rhythms that satisfy the rhythmic oddity property. See [1]
///
///   [1]: Simha Arom. African Polyphony and Polyrhythm.
///        Cambridge University Press, Cambridge, England, 1991.
pub fn hop_jump(_: &mut ExtState, state: &mut InterpState) -> InterpResult {
    let hopsize = try!(state.pop_num()) as usize;
    let pulses = try!(state.pop_num()) as usize;
    let onsets = try!(state.pop_num()) as usize;

    if onsets * hopsize >= pulses {
        return Err(RuntimeErr::InvalidArgs);
    }

    let mut rhythm: Vec<u8> = vec![0; pulses];
    let mut onset = 0;
    let mut pulse = 0;

    loop {
        if onset >= onsets {
            break;
        }
        let value = rhythm[pulse];
        let opposing = pulse + (pulses / 2);
        if value == 0 {
            rhythm[pulse] = 1;
            if opposing < pulses {
                rhythm[opposing] = 2;
            }
            onset += 1;
            pulse = onset * hopsize;
        } else {
            pulse += 1
        }
    }

    let start = state.heap_len();
    for value in &mut rhythm {
        let value = *value;
        if value == 2 {
            state.heap_push(Value::Number(0.0));
        } else {
            state.heap_push(Value::Number(value as f64));
        }
    }

    let len = state.heap_len();
    try!(state.push(Value::Pair(start, len)));
    Ok(None)
}

/// Define a list of simultanious events
pub fn simul(_: &mut ExtState, state: &mut InterpState) -> InterpResult {
    let (start, end) = try!(state.pop_pair());
    try!(state.push(Value::Tuple(start, end)));
    Ok(None)
}

/// Define a list of track functions
pub fn tracks(seq: &mut ExtState, state: &mut InterpState) -> InterpResult {
    let (start, end) = try!(state.pop_pair());
    for (i, ptr) in (start..end).enumerate() {
        let sym = try!(try!(state.heap_get(ptr)).as_sym());
        seq.tracks.push((i, sym));
    }
    Ok(None)
}

fn subdivide(state: &mut InterpState,
             dur: f64,
             dest: Destination)
             -> Result<Vec<Event>, RuntimeErr> {
    let mut output = Vec::new();

    let mut visit: Vec<(f64, f64, Value)> = Vec::new();
    visit.push((0.0, dur, try!(state.pop())));

    while let Some((onset, dur, val)) = visit.pop() {
        match val {
            Value::Curve(points) => {
                let event = Event {
                    dest: dest,
                    onset: onset,
                    dur: dur,
                    value: EventValue::Curve(points),
                };
                output.push(event);
            }
            Value::Null => (),
            Value::Number(val) => {
                let event = Event {
                    dest: dest,
                    onset: onset,
                    dur: dur,
                    value: EventValue::Trigger(val),
                };
                output.push(event);
            }
            Value::Pair(start, end) => {
                let interval = dur / (end - start) as f64;
                let mut onset = onset;
                for n in start..end {
                    visit.push((onset, interval, try!(state.heap_get(n))));
                    onset += interval;
                }
            }
            Value::Tuple(start, end) => {
                for n in start..end {
                    visit.push((onset, dur, try!(state.heap_get(n))));
                }
            }
            _ => return Err(RuntimeErr::InvalidArgs),
        }
    }

    Ok(output)
}

/// Output midi events
pub fn midi_out(seq: &mut ExtState, state: &mut InterpState) -> InterpResult {
    let extra = try!(state.pop_num()) as u8;
    let chan = try!(state.pop_num()) as u8;
    let dur = try!(state.pop_num());
    let mut events =
        try!(subdivide(state, dur, Destination::Midi(chan, extra)));
    seq.duration = dur;
    seq.events.append(&mut events);
    Ok(None)
}

/// Create a bezier curve from a linear ramp
pub fn linear(_: &mut ExtState, state: &mut InterpState) -> InterpResult {
    let (start, end) = try!(state.pop_pair());
    if end - start != 2 {
        return Err(RuntimeErr::InvalidArgs);
    }

    let c0 = try!(try!(state.heap_get(start)).as_num());
    let c1 = try!(try!(state.heap_get(start + 1)).as_num());
    let curve = path_to_curve(&[0.0, c0 as f64], &[1.0, c1 as f64]);
    try!(state.push(Value::Curve(curve)));
    Ok(None)
}

/// Gray code number encoding
pub fn gray_code(_: &mut ExtState, state: &mut InterpState) -> InterpResult {
    let num = try!(state.pop_num()) as i64;
    let num = (num >> 1) ^ num;
    try!(state.push(Value::Number(num as f64)));
    Ok(None)
}

/// Encode a number into a binary list
pub fn bin_list(_: &mut ExtState, state: &mut InterpState) -> InterpResult {
    let num = try!(state.pop_num()) as i64;
    let n = try!(state.pop_num()) as i64;

    let start = state.heap_len();
    for i in 0..n {
        let val = if num & (1 << i) > 0 {
            Value::Number(1.0)
        } else {
            Value::Null
        };
        state.heap_push(val);
    }

    let len = state.heap_len();
    try!(state.push(Value::Pair(start, len)));
    Ok(None)
}

/// Puts the current cycle revision onto the stack
pub fn revision(seq: &mut ExtState, state: &mut InterpState) -> InterpResult {
    try!(state.push(Value::Number(seq.revision as f64)));
    Ok(None)
}

pub fn channels(seq: &mut ExtState, state: &mut InterpState) -> InterpResult {
    seq.audio.settings.channels = try!(state.pop_num()) as f32;
    Ok(None)
}

pub fn block_size(seq: &mut ExtState, state: &mut InterpState) -> InterpResult {
    seq.audio.settings.block_size = try!(state.pop_num()) as f32;
    Ok(None)
}

pub fn sample_rate(seq: &mut ExtState,
                   state: &mut InterpState)
                   -> InterpResult {
    seq.audio.settings.sample_rate = try!(state.pop_num()) as f32;
    Ok(None)
}

pub fn wave_table(seq: &mut ExtState, state: &mut InterpState) -> InterpResult {
    let len = try!(state.pop_num());
    let wave = try!(try!(state.pop()).as_sym());
    let id = try!(try!(state.pop()).as_sym());

    let mut synth = WaveTable::new(len as usize);
    if wave == hash_str("noise") {
        synth.noise();
    } else if wave == hash_str("sine") {
        synth.sine();
    } else {
        return Err(RuntimeErr::InvalidArgs);
    }

    seq.audio.synths.insert(id, Rc::new(synth));
    Ok(None)
}

/// Output synth events
pub fn synth_out(seq: &mut ExtState, state: &mut InterpState) -> InterpResult {
    let param = try!(try!(state.pop()).as_sym());
    let synth = try!(try!(state.pop()).as_sym());
    let dur = try!(state.pop_num());
    let mut events =
        try!(subdivide(state, dur, Destination::Synth(synth, param)));
    seq.duration = dur;
    seq.events.append(&mut events);
    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn repeat_keyword() {
        let mut state = InterpState::new();
        let mut seq = ExtState::new();
        state.call(0, 1).unwrap();
        state.push(Value::Number(12.0)).unwrap();
        state.push(Value::Number(3.0)).unwrap();
        repeat(&mut seq, &mut state).unwrap();
        assert_eq!(state.pop_num().unwrap(), 12.0);
        assert_eq!(state.pop_num().unwrap(), 12.0);
        assert_eq!(state.pop_num().unwrap(), 12.0);
        assert_eq!(state.pop().is_err(), true);
    }

    #[test]
    fn every_keyword_true() {
        let mut state = InterpState::new();
        let mut seq = ExtState::new();
        seq.revision = 3;
        state.call(0, 1).unwrap();
        state.push(Value::Number(3.14)).unwrap();
        state.push(Value::Number(2.17)).unwrap();
        state.push(Value::Number(3.0)).unwrap();
        every(&mut seq, &mut state).unwrap();
        assert_eq!(state.pop_num().unwrap(), 3.14);
        assert_eq!(state.pop().is_err(), true);
    }

    #[test]
    fn every_keyword_false() {
        let mut state = InterpState::new();
        let mut seq = ExtState::new();
        seq.revision = 3;
        state.call(0, 1).unwrap();
        state.push(Value::Number(3.14)).unwrap();
        state.push(Value::Number(2.17)).unwrap();
        state.push(Value::Number(4.0)).unwrap();
        every(&mut seq, &mut state).unwrap();
        assert_eq!(state.pop_num().unwrap(), 2.17);
        assert_eq!(state.pop().is_err(), true);
    }

    #[test]
    fn reverse_keyword() {
        let mut state = InterpState::new();
        let mut seq = ExtState::new();
        state.call(0, 1).unwrap();
        state.heap_push(Value::Number(1.0));
        state.heap_push(Value::Number(2.0));
        state.heap_push(Value::Number(3.0));
        state.push(Value::Pair(0, 3)).unwrap();
        reverse(&mut seq, &mut state).unwrap();
        let out = state.heap_slice_mut(0, 3).unwrap();
        assert_eq!(out[0].as_num().unwrap(), 3.0);
        assert_eq!(out[1].as_num().unwrap(), 2.0);
        assert_eq!(out[2].as_num().unwrap(), 1.0);
    }

    #[test]
    fn rotate_keyword() {
        let mut state = InterpState::new();
        let mut seq = ExtState::new();
        state.call(0, 1).unwrap();
        state.heap_push(Value::Number(1.0));
        state.heap_push(Value::Number(2.0));
        state.heap_push(Value::Number(3.0));
        state.push(Value::Pair(0, 3)).unwrap();
        state.push(Value::Number(1.0)).unwrap();
        rotate(&mut seq, &mut state).unwrap();
        let out = state.heap_slice_mut(0, 3).unwrap();
        assert_eq!(out[0].as_num().unwrap(), 3.0);
        assert_eq!(out[1].as_num().unwrap(), 1.0);
        assert_eq!(out[2].as_num().unwrap(), 2.0);
    }

    #[test]
    fn test_simultaneous_events() {
        let mut state = InterpState::new();
        let mut seq = ExtState::new();
        state.call(0, 1).unwrap();
        state.heap_push(Value::Number(1.0));
        state.heap_push(Value::Number(2.0));
        state.heap_push(Value::Number(3.0));
        state.push(Value::Pair(0, 3)).unwrap();
        simul(&mut seq, &mut state).unwrap();
        state.push(Value::Number(1000.0)).unwrap();
        state.push(Value::Number(0.0)).unwrap();
        state.push(Value::Number(127.0)).unwrap();
        midi_out(&mut seq, &mut state).unwrap();

        assert_eq!(seq.events,
                   [Event {
                        dest: Destination::Midi(0, 127),
                        onset: 0.0,
                        dur: 1000.0,
                        value: EventValue::Trigger(3.0),
                    },
                    Event {
                        dest: Destination::Midi(0, 127),
                        onset: 0.0,
                        dur: 1000.0,
                        value: EventValue::Trigger(2.0),
                    },
                    Event {
                        dest: Destination::Midi(0, 127),
                        onset: 0.0,
                        dur: 1000.0,
                        value: EventValue::Trigger(1.0),
                    }]);
    }

    #[test]
    fn test_binlist() {
        let mut state = InterpState::new();
        let mut seq = ExtState::new();
        state.call(0, 1).unwrap();
        state.push(Value::Number(5.0)).unwrap();
        state.push(Value::Number(12.0)).unwrap();
        bin_list(&mut seq, &mut state).unwrap();
        assert_eq!(state.heap_len(), 5);
        let out = state.heap_slice_mut(0, 5).unwrap();
        assert_eq!(out,
                   &[Value::Null,
                     Value::Null,
                     Value::Number(1.0),
                     Value::Number(1.0),
                     Value::Null]);
    }

    #[test]
    fn test_graycode() {
        let mut state = InterpState::new();
        let mut seq = ExtState::new();
        state.call(0, 1).unwrap();
        state.push(Value::Number(12.0)).unwrap();
        gray_code(&mut seq, &mut state).unwrap();
        assert_eq!(state.pop_num().unwrap() as i64, 10);
    }

    #[test]
    fn test_rev() {
        let mut state = InterpState::new();
        let mut seq = ExtState::new();
        seq.revision = 99;
        state.call(0, 1).unwrap();
        revision(&mut seq, &mut state).unwrap();
        assert_eq!(state.pop_num().unwrap(), 99.0);
    }
}
