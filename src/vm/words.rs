use rand::{Rng, SeedableRng, StdRng};
use std::collections::BTreeSet;
use std::iter;
use std::rc::Rc;

use byteorder::{LittleEndian, WriteBytesExt};

use err::RuntimeErr;

use super::filters::Filter;
use super::interp::{InterpResult, InterpState, Value};
use super::markov::MarkovFilter;
use super::math::path_to_curve;
use super::midi::MidiVelocityMapper;
use super::msgs::{Destination, Event, EventValue};
use super::pitch::PitchQuantizeFilter;

pub type ExtKeyword = fn(&mut ExtState, &mut InterpState) -> InterpResult;

#[derive(Clone)]
pub struct Track {
    pub id: usize,
    pub func: u64,
    pub filters: Vec<Rc<Filter>>,
}

impl Track {
    pub fn new(id: usize, func: u64) -> Track {
        Track {
            id: id,
            func: func,
            filters: Vec::new(),
        }
    }
}

#[derive(Clone)]
pub struct ExtState {
    pub revision: usize,
    pub events: Vec<Event>,
    pub tracks: Vec<Track>,
    pub duration: f64,
    pub rng: StdRng,
}

impl ExtState {
    pub fn new() -> ExtState {
        ExtState {
            revision: 0,
            events: Vec::new(),
            tracks: Vec::new(),
            duration: 0.0,
            rng: StdRng::from_seed(&[0, 0, 0, 0]),
        }
    }
}

/// Repeat a value 'n' times
pub fn repeat(_: &mut ExtState, state: &mut InterpState) -> InterpResult {
    let times = try!(state.pop_num()) as usize;
    let val = try!(state.pop());
    for _ in 0..times {
        try!(state.push(val.clone()));
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

/// Seed the random number generator
pub fn rand_seed(seq: &mut ExtState, state: &mut InterpState) -> InterpResult {
    let seed = try!(state.pop_num()) as i64;
    let mut wtr = vec![];
    if wtr.write_i64::<LittleEndian>(seed).is_err() {
        return Err(RuntimeErr::InvalidArgs);
    }
    let seed: Vec<usize> = wtr.iter().map(|n| *n as usize).collect();
    seq.rng.reseed(seed.as_slice());
    Ok(None)
}

/// Push a random integer, within a range, onto the stack
pub fn rand_range(seq: &mut ExtState, state: &mut InterpState) -> InterpResult {
    let max = try!(state.pop_num()) as i64;
    let min = try!(state.pop_num()) as i64;
    let val = seq.rng.gen_range(min, max);
    try!(state.push(Value::Number(val as f64)));
    Ok(None)
}

/// Shuffle a list, leaving it on the stack
pub fn shuffle(seq: &mut ExtState, state: &mut InterpState) -> InterpResult {
    let (start, end) = try!(state.last_pair());
    let slice = try!(state.heap_slice_mut(start, end));
    seq.rng.shuffle(slice);
    Ok(None)
}

/// Rotate a list
pub fn rotate(_: &mut ExtState, state: &mut InterpState) -> InterpResult {
    let amount = try!(state.pop_num()) as usize;
    let (start, end) = try!(state.last_pair());

    let lst = try!(state.heap_slice_mut(start, end)).to_vec();
    let len = lst.len();
    let (a, b) = lst.split_at(len - (amount % len));
    let mut out = Vec::new();
    out.extend_from_slice(b);
    out.extend_from_slice(a);
    let slice = try!(state.heap_slice_mut(start, end));
    slice.clone_from_slice(&out);
    Ok(None)
}

/// Randomly set values to rests in a list
pub fn degrade(seq: &mut ExtState, state: &mut InterpState) -> InterpResult {
    let (start, end) = try!(state.last_pair());
    let lst = try!(state.heap_slice_mut(start, end));
    for item in lst {
        if seq.rng.gen() {
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
            state.heap_push(Value::Number(f64::from(value)));
        }
    }

    let len = state.heap_len();
    try!(state.push(Value::Pair(start, len)));
    Ok(None)
}

/// Output midi events
pub fn midi_out(seq: &mut ExtState, state: &mut InterpState) -> InterpResult {
    let chan = try!(state.pop_num()) as u8;
    let dur = try!(state.pop_num());

    let mut output = Vec::new();

    let mut visit: Vec<(f64, f64, Value)> = Vec::new();
    visit.push((0.0, dur, try!(state.pop())));

    while let Some((onset, dur, val)) = visit.pop() {
        match val {
            Value::Curve(points) => {
                let event = Event {
                    dest: Destination::Midi(chan, 0),
                    onset: onset,
                    dur: dur,
                    value: EventValue::Curve(points),
                };
                output.push(event);
            }
            Value::Null => (),
            Value::Number(val) => {
                let event = Event {
                    dest: Destination::Midi(chan, 127),
                    onset: onset,
                    dur: dur,
                    value: EventValue::Trigger(val),
                };
                output.push(event);
            }
            Value::Expr(start, end) => {
                let interval = dur / (end - start) as f64;
                let mut onset = onset;
                for n in start..end {
                    visit.push((onset, interval, try!(state.heap_get(n))));
                    onset += interval;
                }
            }
            Value::Group(start, end) => {
                for n in start..end {
                    visit.push((onset, dur, try!(state.heap_get(n))));
                }
            }
            Value::Pair(start, end) => {
                let len = end - start;
                if len == 0 || len > 3 {
                    return Err(RuntimeErr::InvalidArgs);
                }

                let (value, default) = match try!(state.heap_get(start)) {
                    Value::Curve(points) => (EventValue::Curve(points), 0),
                    Value::Number(pitch) => (EventValue::Trigger(pitch), 127),
                    _ => return Err(RuntimeErr::InvalidArgs),
                };

                let dest = Destination::Midi(
                    if len == 3 {
                        try!(try!(state.heap_get(start + 2)).as_num()) as u8
                    } else {
                        chan
                    },
                    if len == 2 {
                        try!(try!(state.heap_get(start + 1)).as_num()) as u8
                    } else {
                        default
                    },
                );

                output.push(Event {
                    dest: dest,
                    onset: onset,
                    dur: dur,
                    value: value,
                });
            }
            _ => return Err(RuntimeErr::InvalidArgs),
        }
    }

    seq.duration = dur;
    seq.events.append(&mut output);
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

/// Assign a markov filter to a track
pub fn markov_filter(seq: &mut ExtState,
                     state: &mut InterpState)
                     -> InterpResult {
    let capacity = try!(state.pop_num()) as usize;
    let order = try!(state.pop_num()) as usize;
    let sym = try!(try!(state.pop()).as_sym());

    if order == 0 || capacity == 0 {
        return Err(RuntimeErr::InvalidArgs);
    }

    match seq.tracks.iter_mut().find(
        |ref mut track| track.func == sym,
    ) {
        Some(track) => {
            let filter = MarkovFilter::new(order, capacity, seq.rng);
            track.filters.push(Rc::new(filter));
            Ok(None)
        }
        None => Err(RuntimeErr::InvalidArgs),
    }
}

pub fn pitch_quantize_filter(seq: &mut ExtState,
                             state: &mut InterpState)
                             -> InterpResult {
    let scale = try!(try!(state.pop()).as_sym());
    let octave = try!(state.pop_num()) as usize;
    let key = try!(try!(state.pop()).as_sym());
    let sym = try!(try!(state.pop()).as_sym());

    let track = match seq.tracks.iter_mut().find(
        |ref mut track| track.func == sym,
    ) {
        Some(track) => track,
        None => return Err(RuntimeErr::InvalidArgs),
    };

    let filter = match PitchQuantizeFilter::new(key, octave, scale) {
        Some(filter) => filter,
        None => return Err(RuntimeErr::InvalidArgs),
    };

    track.filters.push(Rc::new(filter));
    Ok(None)
}

pub fn midi_velocity_filter(seq: &mut ExtState,
                            state: &mut InterpState)
                            -> InterpResult {
    let param = try!(try!(state.pop()).as_sym());
    let device = try!(try!(state.pop()).as_sym());
    let name = try!(try!(state.pop()).as_sym());

    let track = match seq.tracks.iter_mut().find(
        |ref mut track| track.func == name,
    ) {
        Some(track) => track,
        None => return Err(RuntimeErr::InvalidArgs),
    };

    match MidiVelocityMapper::new(device, param) {
        Some(filter) => track.filters.push(Rc::new(filter)),
        None => return Err(RuntimeErr::InvalidArgs),
    };

    Ok(None)
}

/// Construct a continuous integer sequence from `a` to `b`
pub fn range(_: &mut ExtState, state: &mut InterpState) -> InterpResult {
    let b = try!(state.pop_num()) as usize;
    let a = try!(state.pop_num()) as usize;

    let start = state.heap_len();
    for i in a..b {
        state.heap_push(Value::Number(i as f64));
    }

    let end = state.heap_len();
    try!(state.push(Value::Pair(start, end)));
    Ok(None)
}

/// Apply a residual class to an integer sequence
pub fn sieve(_: &mut ExtState, state: &mut InterpState) -> InterpResult {
    let (modulus, shift) = try!(state.pop_pair());
    let (start, end) = try!(state.pop_pair());

    let next_start = state.heap_len();
    for ptr in start..end {
        let val = try!(try!(state.heap_get(ptr)).as_num()) as usize;
        if val % modulus == shift {
            state.heap_push(Value::Number(val as f64));
        }
    }

    let next_end = state.heap_len();
    try!(state.push(Value::Pair(next_start, next_end)));
    Ok(None)
}

/// Return a new list containing the difference between consecutive elements
pub fn inter_onset(_: &mut ExtState, state: &mut InterpState) -> InterpResult {
    let (start, end) = try!(state.pop_pair());
    let count = end - start;
    let heap_start = state.heap_len();

    if count == 1 {
        state.heap_push(Value::Number(0.0));
    } else if count > 1 {
        for i in start..end - 1 {
            let curr = try!(try!(state.heap_get(i)).as_num());
            let next = try!(try!(state.heap_get(i + 1)).as_num());
            state.heap_push(Value::Number(next - curr as f64));
        }
    }

    let end = state.heap_len();
    try!(state.push(Value::Pair(heap_start, end)));
    Ok(None)
}

/// Return a binary onset representation of a list
pub fn onsets(_: &mut ExtState, state: &mut InterpState) -> InterpResult {
    let (start, end) = try!(state.pop_pair());
    let b = try!(state.pop_num()) as usize;
    let a = try!(state.pop_num()) as usize;

    let heap_start = state.heap_len();

    if end - start != 0 {
        let mut out = iter::repeat(0).take(b - a).collect::<Vec<_>>();
        for i in start..end {
            let val = try!(try!(state.heap_get(i)).as_num()) as usize;
            if a <= val && val < b {
                out[val - a] = 1;
            }
        }
        for val in out {
            state.heap_push(Value::Number(f64::from(val)));
        }
    }

    let heap_end = state.heap_len();
    try!(state.push(Value::Pair(heap_start, heap_end)));
    Ok(None)
}

pub fn _pop_set(state: &mut InterpState)
                -> Result<BTreeSet<usize>, RuntimeErr> {
    let (start, end) = try!(state.pop_pair());
    let mut output = BTreeSet::new();

    for ptr in start..end {
        let val = try!(try!(state.heap_get(ptr)).as_num()) as usize;
        output.insert(val);
    }

    Ok(output)
}

/// Perform the intersection ('or') of two lists
pub fn intersection(_: &mut ExtState, state: &mut InterpState) -> InterpResult {
    let a = try!(_pop_set(state));
    let b = try!(_pop_set(state));
    let vals: Vec<usize> = a.intersection(&b).cloned().collect();

    let start = state.heap_len();
    for val in vals {
        state.heap_push(Value::Number(val as f64));
    }

    let end = state.heap_len();
    try!(state.push(Value::Pair(start, end)));
    Ok(None)
}

/// Perform the union ('and') of two lists
pub fn union(_: &mut ExtState, state: &mut InterpState) -> InterpResult {
    let a = try!(_pop_set(state));
    let b = try!(_pop_set(state));
    let vals: Vec<usize> = a.union(&b).cloned().collect();

    let start = state.heap_len();
    for val in vals {
        state.heap_push(Value::Number(val as f64));
    }

    let end = state.heap_len();
    try!(state.push(Value::Pair(start, end)));
    Ok(None)
}

/// Perform the symmetric difference ('xor') between two lists
pub fn symmetric_difference(_: &mut ExtState,
                            state: &mut InterpState)
                            -> InterpResult {
    let a = try!(_pop_set(state));
    let b = try!(_pop_set(state));
    let vals: Vec<usize> = a.symmetric_difference(&b).cloned().collect();

    let start = state.heap_len();
    for val in vals {
        state.heap_push(Value::Number(val as f64));
    }

    let end = state.heap_len();
    try!(state.push(Value::Pair(start, end)));
    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn random_keywords() {
        let mut state = InterpState::new();
        let mut seq = ExtState::new();
        state.call(0, 1).unwrap();
        state.push(Value::Number(3.0)).unwrap();
        rand_seed(&mut seq, &mut state).unwrap();
        state.push(Value::Number(0.0)).unwrap();
        state.push(Value::Number(100.0)).unwrap();
        rand_range(&mut seq, &mut state).unwrap();
        assert_eq!(state.pop_num().unwrap(), 31.0);
    }

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
        state.push(Value::Group(0, 3)).unwrap();
        state.push(Value::Number(1000.0)).unwrap();
        state.push(Value::Number(0.0)).unwrap();
        midi_out(&mut seq, &mut state).unwrap();

        assert_eq!(
            seq.events,
            [
                Event {
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
                },
            ]
        );
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
        assert_eq!(
            out,
            &[
                Value::Null,
                Value::Null,
                Value::Number(1.0),
                Value::Number(1.0),
                Value::Null,
            ]
        );
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

    #[test]
    fn test_range() {
        let mut state = InterpState::new();
        let mut seq = ExtState::new();
        state.call(0, 1).unwrap();
        state.push(Value::Number(2.0)).unwrap();
        state.push(Value::Number(6.0)).unwrap();
        range(&mut seq, &mut state).unwrap();
        let out = state.heap_slice_mut(0, 4).unwrap();
        assert_eq!(
            out,
            &[
                Value::Number(2.0),
                Value::Number(3.0),
                Value::Number(4.0),
                Value::Number(5.0),
            ]
        );
    }
}
