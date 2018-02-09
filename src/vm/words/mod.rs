mod bin;
mod curve;
mod debug;
mod fx;
mod list;
mod math;
mod midi;
mod prob;
mod rhythm;
mod set;
mod stack;
mod track;

use std::collections::HashMap;

use vm::types::Keyword;


type Module = HashMap<&'static str, Keyword>;

fn bin(words: &mut Module) {
    words.insert("bin_list", bin::bin_list);
    words.insert("gray_code", bin::gray_code);
}

fn curve(words: &mut Module) {
    words.insert("linear", curve::linear);
}

fn debug(words: &mut Module) {
    words.insert("print", debug::print);
    words.insert("print_heap", debug::print_heap);
}

fn fx(words: &mut Module) {
    words.insert("pitch_quantize_filter", fx::pitch_quantize_filter);
    words.insert("markov_filter", fx::markov_filter);
    words.insert("midi_velocity_filter", fx::midi_velocity_filter);
}

fn list(words: &mut Module) {
    words.insert("cycle", list::cycle);
    words.insert("degrade", list::degrade);
    words.insert("every", list::every);
    words.insert("palindrome", list::palindrome);
    words.insert("range", list::range);
    words.insert("repeat", list::repeat);
    words.insert("reverse", list::reverse);
    words.insert("rotate", list::rotate);
    words.insert("shuffle", list::shuffle);
}

fn math(words: &mut Module) {
    words.insert("add", math::add);
    words.insert("divide", math::divide);
    words.insert("modulo", math::modulo);
    words.insert("multiply", math::multiply);
    words.insert("subtract", math::subtract);
}

fn midi(words: &mut Module) {
    words.insert("midi_out", midi::midi_out);
}

fn prob(words: &mut Module) {
    words.insert("rand_range", prob::rand_range);
    words.insert("rand_seed", prob::rand_seed);
}

fn rhythm(words: &mut Module) {
    words.insert("hop_jump", rhythm::hop_jump);
    words.insert("inter_onset", rhythm::inter_onset);
    words.insert("onsets", rhythm::onsets);
}

fn set(words: &mut Module) {
    words.insert("intersection", set::intersection);
    words.insert("sieve", set::sieve);
    words.insert("symmetric_difference", set::symmetric_difference);
    words.insert("union", set::union);
}

fn stack(words: &mut Module) {
    words.insert("drop", stack::drop);
    words.insert("dup", stack::duplicate);
    words.insert("swap", stack::swap);
}

fn track(words: &mut Module) {
    words.insert("revision", track::revision);
}

pub fn all() -> Module {
    let mut words: HashMap<&'static str, Keyword> = HashMap::new();
    bin(&mut words);
    curve(&mut words);
    debug(&mut words);
    fx(&mut words);
    list(&mut words);
    math(&mut words);
    midi(&mut words);
    prob(&mut words);
    rhythm(&mut words);
    set(&mut words);
    stack(&mut words);
    track(&mut words);
    words
}
