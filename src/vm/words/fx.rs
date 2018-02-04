use std::rc::Rc;

use err::RuntimeErr;
use vm::fx::{MarkovFilter, MidiVelocityMapper, PitchQuantizeFilter};
use vm::interp::InterpState;
use vm::types::{Result, SeqState};


pub fn pitch_quantize_filter(seq: &mut SeqState,
                             state: &mut InterpState)
                             -> Result {
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

/// Assign a markov filter to a track
pub fn markov_filter(seq: &mut SeqState, state: &mut InterpState) -> Result {
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

pub fn midi_velocity_filter(seq: &mut SeqState,
                            state: &mut InterpState)
                            -> Result {
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
