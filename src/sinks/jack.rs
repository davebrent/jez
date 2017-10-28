use std::convert::From;
use std::ops::DerefMut;
use std::sync::mpsc::Receiver;
use std::time::Instant;

use jack::prelude::{AsyncClient, AudioOutPort, AudioOutSpec, Client,
                    JackControl, JackErr, MidiOutPort, MidiOutSpec,
                    NotificationHandler, Port, PortSpec, ProcessHandler,
                    ProcessScope, RawMidi, client_options};

use err::SysErr;
use memory::RingBuffer;
use vm::{AudioBlock, Command};

struct Notifier;
struct Processor {
    ring: RingBuffer<AudioBlock>,
    channel: Receiver<Command>,
    midi_out: Vec<Port<MidiOutSpec>>,
    audio_out: Vec<Port<AudioOutSpec>>,
    block: AudioBlock,
    block_size: usize,
    start: Instant,
}

pub struct Jack {
    _client: AsyncClient<Notifier, Processor>,
}

impl From<JackErr> for SysErr {
    fn from(_: JackErr) -> SysErr {
        SysErr::UnreachableBackend
    }
}

fn make_ports<T>(prefix: &'static str,
                 client: &Client,
                 len: usize)
                 -> Result<Vec<Port<T>>, SysErr>
where
    T: PortSpec,
{
    let mut ports = Vec::with_capacity(len);
    for i in 1..len + 1 {
        let name = format!("{}_{}", prefix, i);
        match client.register_port(name.as_str(), T::default()) {
            Ok(port) => ports.push(port),
            Err(_) => return Err(SysErr::UnreachableBackend),
        };
    }
    Ok(ports)
}

impl NotificationHandler for Notifier {
    // TODO: Handle settings changes
}

impl Processor {
    fn process_msgs(&mut self,
                    client: &Client,
                    ps: &ProcessScope)
                    -> Result<(), SysErr> {
        let mut ports: Vec<MidiOutPort> = self.midi_out
            .iter_mut()
            .map(|port| MidiOutPort::new(port, ps))
            .collect();

        while let Ok(msg) = self.channel.try_recv() {
            let time = Instant::now() - self.start;

            match msg {
                Command::AudioSettings(channels, block_size, _) => {
                    self.block_size = block_size;
                    if channels != self.audio_out.len() {
                        self.audio_out =
                            try!(make_ports("audioout", client, channels));
                    }
                }
                Command::MidiNoteOn(chn, pitch, vel) => {
                    let midi = RawMidi {
                        time: 0,
                        bytes: &[144 + chn, pitch, vel],
                    };
                    ports[0].write(&midi).unwrap();
                }
                Command::MidiNoteOff(chn, pitch) => {
                    let midi = RawMidi {
                        time: 0,
                        bytes: &[128 + chn, pitch, 0],
                    };
                    ports[0].write(&midi).unwrap();
                }
                Command::MidiCtl(chn, ctl, val) => {
                    let midi = RawMidi {
                        time: 0,
                        bytes: &[176 + chn, ctl, val],
                    };
                    ports[0].write(&midi).unwrap();
                }
                _ => (),
            }
        }

        Ok(())
    }

    fn process_audio(&mut self, ps: &ProcessScope) -> JackControl {
        let mut ports: Vec<AudioOutPort> = self.audio_out
            .iter_mut()
            .map(|port| AudioOutPort::new(port, ps))
            .collect();

        let channels = ports.len();
        if channels == 0 {
            // No audio settings have been recieved yet
            return JackControl::Continue;
        }

        // Clear the temporary block and try and read from the ring buffer
        self.block.clear(self.block_size);
        match self.ring.advance_read() {
            None => {
                // Output silence if no block is available
                let dest = self.block.as_slice();
                for port in &mut ports {
                    port.deref_mut().copy_from_slice(&dest);
                }
                JackControl::Continue
            }
            Some(block) => {
                // If the block is available, de-interleave the buffer into the
                // temporary block
                let mut dest = self.block.as_mut_slice();
                let src = block.as_slice();

                for channel in 0..channels {
                    for i in 0..self.block_size {
                        dest[i] = src[(i * channels) + channel];
                    }

                    let buffer = ports[channel].deref_mut();
                    // Check that Jack is running at the same block size before
                    // outputting any data
                    if buffer.len() == self.block_size {
                        buffer.copy_from_slice(&dest);
                    } else {
                        return JackControl::Quit;
                    }
                }

                JackControl::Continue
            }
        }
    }
}

impl ProcessHandler for Processor {
    fn process(&mut self, client: &Client, ps: &ProcessScope) -> JackControl {
        // Messages are processed first, to ensure audio settings are received
        // before any audio is processed
        match self.process_msgs(client, ps) {
            Ok(_) => self.process_audio(ps),
            Err(_) => JackControl::Quit,
        }
    }
}

impl Jack {
    pub fn new(ring: RingBuffer<AudioBlock>,
               channel: Receiver<Command>)
               -> Result<Self, SysErr> {
        let opts = client_options::NO_START_SERVER;
        let (client, _) = try!(Client::new("jez", opts));
        let notifier = Notifier {};
        let processor = Processor {
            ring: ring,
            block: AudioBlock::new(64),
            block_size: 64,
            channel: channel,
            midi_out: try!(make_ports("midiout", &client, 1)),
            audio_out: Vec::new(),
            start: Instant::now(),
        };

        Ok(Jack {
            _client: try!(AsyncClient::new(client, notifier, processor)),
        })
    }
}
