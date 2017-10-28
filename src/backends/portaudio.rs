use std::sync::mpsc::Receiver;
use std::thread;

use err::SysErr;
use memory::RingBuffer;
use vm::{AudioBlock, Command};

use portaudio as pa;

impl From<pa::Error> for SysErr {
    fn from(_: pa::Error) -> SysErr {
        SysErr::UnreachableBackend
    }
}

#[derive(Debug)]
struct RuntimeSettings {
    channels: i32,
    block_size: u32,
    sample_rate: f64,
}

fn audio_callback(ring: &RingBuffer<AudioBlock>,
                  _: &RuntimeSettings,
                  args: pa::OutputStreamCallbackArgs<f32>)
                  -> pa::StreamCallbackResult {
    match ring.advance_read() {
        None => {
            // Output silence when no block is available
            for i in 0..args.frames {
                args.buffer[i] = 0.0;
            }
            pa::Continue
        }
        Some(block) => {
            // Output interleaved samples
            let src = block.as_slice();
            args.buffer.copy_from_slice(&src);
            pa::Continue
        }
    }
}

pub struct Portaudio;

impl Portaudio {
    pub fn new(ring: RingBuffer<AudioBlock>,
               channel: Receiver<Command>)
               -> Result<Self, SysErr> {
        thread::spawn(move || {
            let pa = match pa::PortAudio::new() {
                Ok(pa) => pa,
                Err(_) => return,
            };

            let mut runtime_settings = RuntimeSettings {
                channels: 0,
                block_size: 0,
                sample_rate: 0.0,
            };

            // Block until audio settings have been received
            while let Ok(msg) = channel.recv() {
                match msg {
                    Command::AudioSettings(channels_,
                                           block_size_,
                                           sample_rate_) => {
                        runtime_settings.channels = channels_ as i32;
                        runtime_settings.block_size = block_size_ as u32;
                        runtime_settings.sample_rate = sample_rate_ as f64;
                        break;
                    }
                    _ => (),
                }
            }

            let settings = match pa.default_output_stream_settings(
                runtime_settings.channels,
                runtime_settings.sample_rate,
                runtime_settings.block_size,
            ) {
                Ok(settings) => settings,
                Err(_) => return,
            };

            let callback =
                move |args| audio_callback(&ring, &runtime_settings, args);

            let mut stream =
                match pa.open_non_blocking_stream(settings, callback) {
                    Ok(stream) => stream,
                    Err(_) => return,
                };

            match stream.start() {
                Ok(_) => (),
                Err(_) => return,
            };

            // Keep the thread alive until the channel is dead? Waste?
            while let Ok(msg) = channel.recv() {
                match msg {
                    _ => continue,
                }
            }

            match stream.stop() {
                _ => return,
            };
        });

        Ok(Portaudio {})
    }
}
