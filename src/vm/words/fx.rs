use crate::vm::fx::{MarkovChain, MidiVelocityMapper, PitchQuantizer};
use crate::vm::interp::InterpState;
use crate::vm::types::{Result, SeqState};

pub fn pitch_quantizer(seq: &mut SeqState, state: &mut InterpState) -> Result {
    let scale = (state.pop()?).as_sym()?;
    let octave = state.pop_num()? as usize;
    let key = (state.pop()?).as_sym()?;
    let sym = (state.pop()?).as_sym()?;

    let track = match seq
        .tracks
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

    track.effects.push(Box::new(fx));
    Ok(None)
}

/// Assign a markov chain to a track
pub fn markov_chain(seq: &mut SeqState, state: &mut InterpState) -> Result {
    let capacity = state.pop_num()? as usize;
    let order = state.pop_num()? as usize;
    let sym = (state.pop()?).as_sym()?;

    if order == 0 || capacity == 0 {
        return Err(error!(InvalidArgs));
    }

    match seq
        .tracks
        .iter_mut()
        .find(|ref mut track| track.func == sym)
    {
        Some(track) => {
            let fx = MarkovChain::new(order, capacity, seq.rng);
            track.effects.push(Box::new(fx));
            Ok(None)
        }
        None => Err(error!(InvalidArgs)),
    }
}

pub fn midi_velocity_mapper(seq: &mut SeqState, state: &mut InterpState) -> Result {
    let param = (state.pop()?).as_sym()?;
    let device = (state.pop()?).as_sym()?;
    let name = (state.pop()?).as_sym()?;

    let track = match seq
        .tracks
        .iter_mut()
        .find(|ref mut track| track.func == name)
    {
        Some(track) => track,
        None => return Err(error!(InvalidArgs)),
    };

    match MidiVelocityMapper::new(device, param) {
        Some(fx) => track.effects.push(Box::new(fx)),
        None => return Err(error!(InvalidArgs)),
    };

    Ok(None)
}
