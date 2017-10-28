use std::collections::HashMap;
use std::fmt::Debug;
use std::rc::Rc;
use std::sync::mpsc::Sender;
use std::time::Duration;

use super::math::dur_to_millis;
use super::msgs::{Command, Destination, Event, EventValue};
use super::ring::RingBuffer;

pub type Sample = f32;

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct AudioSettings {
    pub channels: f32,
    pub block_size: f32,
    pub sample_rate: f32,
}

impl AudioSettings {
    pub fn new() -> AudioSettings {
        AudioSettings {
            channels: 2.0,
            block_size: 128.0,
            sample_rate: 44100.0,
        }
    }
}

#[derive(Clone, Debug)]
pub struct AudioBlock {
    data: Vec<Sample>,
}

impl AudioBlock {
    pub fn new(len: usize) -> AudioBlock {
        let mut data = Vec::with_capacity(len);
        data.resize(len, 0.0);
        AudioBlock { data: data }
    }

    pub fn clear(&mut self, len: usize) {
        self.data.resize(len, 0.0);
        for val in &mut self.data {
            *val = 0.0;
        }
    }

    pub fn as_slice(&self) -> &[Sample] {
        self.data.as_slice()
    }

    pub fn as_mut_slice(&mut self) -> &mut [Sample] {
        self.data.as_mut_slice()
    }
}

pub trait Synth: Debug {
    fn set(&mut self, param: u64, value: f64);
    fn configure(&mut self, settings: &AudioSettings);
    fn render(&mut self, block: &mut [Sample], settings: &AudioSettings);
}

#[derive(Clone, Debug)]
pub struct AudioContext {
    pub settings: AudioSettings,
    pub synths: HashMap<u64, Rc<Synth>>,
}

impl AudioContext {
    pub fn new() -> AudioContext {
        AudioContext {
            settings: AudioSettings::new(),
            synths: HashMap::new(),
        }
    }
}

#[derive(Debug)]
pub struct AudioProcessor {
    ring: RingBuffer<AudioBlock>,
    block: AudioBlock,
    last_update: Duration,
    delta: f64,
    context: AudioContext,
    output: Sender<Command>,
}

impl AudioProcessor {
    pub fn new(ring: RingBuffer<AudioBlock>,
               output: Sender<Command>)
               -> AudioProcessor {
        AudioProcessor {
            ring: ring,
            block: AudioBlock::new(64),
            last_update: Duration::new(0, 0),
            delta: 0.0,
            context: AudioContext::new(),
            output: output,
        }
    }

    pub fn configure(&mut self, context: AudioContext) {
        self.context = context;

        let channels = self.context.settings.channels as usize;
        let block_size = self.context.settings.block_size as usize;
        let sample_rate = self.context.settings.sample_rate as usize;

        let cmd = Command::AudioSettings(channels, block_size, sample_rate);
        self.output.send(cmd).ok();

    }

    pub fn process(&mut self, event: Event) {
        let (synth, param) = match event.dest {
            Destination::Synth(synth, param) => (synth, param),
            _ => return,
        };

        // TODO: Maybe the setting of synth params happens pre-render? For ALL
        //       the synths parameters?
        let synth = match self.context.synths.get_mut(&synth) {
            Some(synth) => synth,
            None => return,
        };

        match Rc::get_mut(synth) {
            None => return,
            Some(synth) => {
                match event.value {
                    EventValue::Trigger(f) => synth.set(param, f),
                    _ => return,
                };
            }
        }
    }

    // Return the desired time in milliseconds that `update` should be called
    pub fn interval(&self) -> f64 {
        let sample_rate = self.context.settings.sample_rate;
        let block_size = self.context.settings.block_size;

        // Time in milliseconds between each block
        let interval = 1000.0 / f64::from(sample_rate / block_size);
        // Run 40% quicker, to ensure backend always has enough blocks, with a
        // minimum latency of 0.5ms
        (interval * 0.6).max(0.5)
    }

    pub fn update(&mut self, elapsed: &Duration) {
        if self.last_update == Duration::new(0, 0) {
            for synth in self.context.synths.values_mut() {
                if let Some(synth) = Rc::get_mut(synth) {
                    synth.configure(&self.context.settings)
                }
            }
        }

        let delta = match elapsed.checked_sub(self.last_update) {
            Some(dur) => dur,
            None => Duration::new(0, 0),
        };

        self.last_update = *elapsed;
        self.delta += dur_to_millis(&delta);

        // Calculate the number of blocks that should be rendered for this time
        let num_blocks = (self.delta / self.interval()).floor() as usize;
        if num_blocks != 0 {
            self.delta = 0.0;
            for _ in 0..num_blocks {
                self.render();
            }
        }
    }

    fn render(&mut self) {
        let block_size = self.context.settings.block_size as usize;
        let channels = self.context.settings.channels as usize;
        let capacity = block_size * channels;

        // Try and get a writable block from the ring buffer
        let block = self.ring.advance_write();
        if block.is_none() {
            return;
        }

        let mut output = block.unwrap();
        output.clear(capacity);
        self.block.clear(capacity);

        let output = output.as_mut_slice();
        let temp = self.block.as_mut_slice();

        // Render all synths and sum the result into the writable block
        for synth in self.context.synths.values_mut() {
            if let Some(synth) = Rc::get_mut(synth) {
                synth.render(temp, &self.context.settings);
                for i in 0..capacity {
                    output[i] += temp[i];
                }
            }
        }
    }
}
