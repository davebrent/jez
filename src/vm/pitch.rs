use std::collections::HashMap;

use lang::hash_str;

use super::filters::Filter;
use super::msgs::{Event, EventValue};

#[derive(Clone, Debug)]
pub struct PitchQuantizeFilter {
    key: usize,
    scale: Vec<usize>,
    octave: usize,
}

impl PitchQuantizeFilter {
    pub fn new(key: u64,
               octave: usize,
               scale: u64)
               -> Option<PitchQuantizeFilter> {
        let mut keys = HashMap::new();
        keys.insert(hash_str("C"), 0);
        keys.insert(hash_str("C#"), 1);
        keys.insert(hash_str("D"), 2);
        keys.insert(hash_str("D#"), 3);
        keys.insert(hash_str("Eb"), 4);
        keys.insert(hash_str("E"), 5);
        keys.insert(hash_str("F"), 6);
        keys.insert(hash_str("F#"), 7);
        keys.insert(hash_str("G"), 8);
        keys.insert(hash_str("Ab"), 9);
        keys.insert(hash_str("Bb"), 10);
        keys.insert(hash_str("B"), 11);

        let (c, cs, d, eb, e, f, fs, g, ab, a, bb, b) =
            (0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11);

        let mut ss = HashMap::new();
        ss.insert(hash_str("natural_minor"), vec![c, d, eb, f, g, ab, bb]);
        ss.insert(hash_str("major"), vec![c, d, e, f, g, a, b]);
        ss.insert(hash_str("dorian"), vec![c, d, eb, f, g, a, bb]);
        ss.insert(hash_str("phrygian"), vec![c, cs, eb, f, g, ab, bb]);
        ss.insert(hash_str("mixolydian"), vec![c, d, e, f, g, a, bb]);
        ss.insert(hash_str("melodic_minor_asc"), vec![c, d, eb, f, g, a, b]);
        ss.insert(hash_str("harmonic_minor"), vec![c, d, eb, f, g, ab, b]);
        ss.insert(hash_str("bebop_dorian"), vec![c, eb, e, f, g, a, bb]);
        ss.insert(hash_str("blues"), vec![c, eb, f, fs, g, bb]);
        ss.insert(hash_str("minor_pentatonic"), vec![c, eb, f, fs, g, bb]);
        ss.insert(hash_str("hungarian_minor"), vec![c, d, eb, fs, g, ab, b]);
        ss.insert(hash_str("ukranian_dorian"), vec![c, d, eb, fs, g, a, bb]);
        ss.insert(hash_str("marva"), vec![c, cs, e, fs, g, a, b]);
        ss.insert(hash_str("todi"), vec![c, cs, eb, fs, g, ab, b]);
        ss.insert(hash_str("whole_tone"), vec![c, d, e, fs, ab, bb]);

        let key = match keys.get(&key) {
            Some(key) => *key,
            None => return None,
        };

        let scale = match ss.get(&scale) {
            Some(scale) => scale.clone(),
            None => return None,
        };

        Some(PitchQuantizeFilter {
            key: key,
            scale: scale,
            octave: octave,
        })
    }

    fn quantize(&self, val: f64) -> f64 {
        let degree = val as usize;
        let pitches: Vec<usize> = self.scale
            .iter()
            .map(|s| {
                let note = s + self.key;
                let shift = note / 12;
                (note % 12) + ((self.octave + shift) * 12)
            })
            .collect();
        pitches[degree % pitches.len()] as f64
    }
}

impl Filter for PitchQuantizeFilter {
    fn apply(&mut self, _: f64, events: &[Event]) -> Vec<Event> {
        let mut output = Vec::with_capacity(events.len());
        for event in events {
            let mut event = *event;
            event.value = match event.value {
                EventValue::Curve(_) => event.value,
                EventValue::Trigger(val) => {
                    EventValue::Trigger(self.quantize(val))
                }
            };
            output.push(event);
        }
        output
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn filter(key: &'static str,
              scale: &'static str,
              octave: usize)
              -> PitchQuantizeFilter {
        PitchQuantizeFilter::new(hash_str(key), octave, hash_str(scale))
            .unwrap()
    }

    #[test]
    fn test_wrap_around_pitches() {
        // D Marva = [D, D#, Eb, F#, Ab, A, B, C#]
        let f = filter("D", "marva", 0);
        assert_eq!(f.quantize(0.0) /* 1st degree */, 2.0 /* D */);
        assert_eq!(f.quantize(6.0) /* 6th degree */, 13.0 /* C# */);
    }
}
