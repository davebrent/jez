use std::rc::Rc;

use vm::fx::{MarkovChain, MidiVelocityMapper, PitchQuantizer};
use vm::interp::InterpState;
use vm::types::{Result, SeqState};

pub fn pitch_quantizer(seq: &mut SeqState, state: &mut InterpState) -> Result {
    let scale = try!(try!(state.pop()).as_sym());
    let octave = try!(state.pop_num()) as usize;
    let key = try!(try!(state.pop()).as_sym());
    let sym = try!(try!(state.pop()).as_sym());

    let track = match seq.tracks
        .iter_mut()
        .find(|ref mut track| track.func == sym)
    {
        Some(track) => track,
        None => return Err(error!(InvalidArgs)),
    };

    let fx = match PitchQuantizer::new(key, octave, scale) {
        Some(fx) => fx,
        None => return Err(error!(InvalidArgs)),
    };

    track.effects.push(Rc::new(fx));
    Ok(None)
}

/// Assign a markov chain to a track
pub fn markov_chain(seq: &mut SeqState, state: &mut InterpState) -> Result {
    let capacity = try!(state.pop_num()) as usize;
    let order = try!(state.pop_num()) as usize;
    let sym = try!(try!(state.pop()).as_sym());

    if order == 0 || capacity == 0 {
        return Err(error!(InvalidArgs));
    }

    match seq.tracks
        .iter_mut()
        .find(|ref mut track| track.func == sym)
    {
        Some(track) => {
            let fx = MarkovChain::new(order, capacity, seq.rng);
            track.effects.push(Rc::new(fx));
            Ok(None)
        }
        None => Err(error!(InvalidArgs)),
    }
}

pub fn midi_velocity_mapper(seq: &mut SeqState, state: &mut InterpState) -> Result {
    let param = try!(try!(state.pop()).as_sym());
    let device = try!(try!(state.pop()).as_sym());
    let name = try!(try!(state.pop()).as_sym());

    let track = match seq.tracks
        .iter_mut()
        .find(|ref mut track| track.func == name)
    {
        Some(track) => track,
        None => return Err(error!(InvalidArgs)),
    };

    match MidiVelocityMapper::new(device, param) {
        Some(fx) => track.effects.push(Rc::new(fx)),
        None => return Err(error!(InvalidArgs)),
    };

    Ok(None)
}
