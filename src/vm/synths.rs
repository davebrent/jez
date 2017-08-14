use std::f32::consts::PI;
use std::f32::EPSILON;
use rand::{Rng, StdRng};

use lang::hash_str;

use super::audio::{AudioSettings, Sample, Synth};

#[derive(Clone, Copy, Debug)]
struct SmoothParam {
    current: f32,
    target: f32,
    step: f32,
    countdown: usize,
    steps_to_target: usize,
}

impl SmoothParam {
    pub fn new(initial: f32) -> SmoothParam {
        SmoothParam {
            current: initial,
            target: initial,
            countdown: 0,
            step: 0.0,
            steps_to_target: 0,
        }
    }

    pub fn reset(&mut self, sample_rate: f32, ramp_ms: f32) {
        let secs = ramp_ms / 1000.0;
        self.steps_to_target = (secs * sample_rate).floor() as usize;
        self.current = self.target;
        self.countdown = 0;
    }

    pub fn set_val(&mut self, val: f32) {
        if (self.target - val).abs() < EPSILON {
            self.target = val;
            self.countdown = self.steps_to_target;

            if self.countdown == 0 {
                self.current = self.target;
            } else {
                let countdown = self.countdown as f32;
                self.step = (self.target - self.current) / countdown;
            }
        }
    }

    pub fn get(&mut self) -> f32 {
        if self.countdown == 0 {
            return self.target;
        }

        self.countdown -= 1;
        self.current += self.step;
        self.current
    }
}

#[derive(Clone, Debug)]
pub struct WaveTable {
    phase: Sample,
    table: Vec<Sample>,
    freq: SmoothParam,
    amp: SmoothParam,
    pan: SmoothParam,
}

impl WaveTable {
    pub fn new(size: usize) -> Self {
        let mut table = Vec::with_capacity(size);
        table.resize(size, 0.0);

        WaveTable {
            phase: 0.0,
            table: table,
            freq: SmoothParam::new(220.0),
            amp: SmoothParam::new(0.5),
            pan: SmoothParam::new(0.5),
        }
    }

    pub fn sine(&mut self) {
        let len = self.table.len();
        for i in 0..len {
            let inc = i as Sample / len as Sample;
            self.table[i] = (inc * PI * 2.0).sin();
        }
    }

    pub fn noise(&mut self) {
        let mut rng = StdRng::new().unwrap();
        for x in &mut self.table {
            *x = rng.gen_range(-1.0, 1.0);
        }
    }
}

impl Synth for WaveTable {
    fn set(&mut self, param: u64, value: f64) {
        let value = value as f32;

        if param == hash_str("freq") {
            self.freq.set_val(value);
        } else if param == hash_str("amp") {
            self.amp.set_val(value);
        } else if param == hash_str("pan") {
            self.pan.set_val(value);
        }
    }

    fn configure(&mut self, settings: &AudioSettings) {
        self.freq.reset(settings.sample_rate, 2.0);
        self.amp.reset(settings.sample_rate, 5.0);
        self.pan.reset(settings.sample_rate, 5.0);
    }

    fn render(&mut self, output: &mut [Sample], settings: &AudioSettings) {
        let block_size = settings.block_size as usize;
        let channels = settings.channels as usize;
        let sample_rate = settings.sample_rate;
        let table_size = self.table.len() as Sample;

        // Based on http://www.musicdsp.org/archive.php?classid=1#16
        for b in 0..block_size {
            let freq = self.freq.get();
            let amp = self.amp.get().sqrt();
            let pan = self.pan.get().sqrt();

            let i = self.phase.floor();
            let alpha = self.phase - i;

            self.phase += table_size / (sample_rate / freq);
            if self.phase >= table_size {
                self.phase -= table_size;
            }

            let i = i as usize;
            let i1 = if i + 1 >= table_size as usize {
                0
            } else {
                i + 1
            };

            let diff = self.table[i1] - self.table[i];
            let samp = self.table[i] + (diff * alpha);
            let samp = samp * amp;

            if channels == 2 {
                // Stereo panning
                output[b * 2] = samp * (1.0 - pan);
                output[(b * 2) + 1] = samp * pan;
            } else {
                // No panning
                for c in 0..channels {
                    output[(b * channels) + c] = samp;
                }
            }
        }
    }
}
