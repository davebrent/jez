use std::sync::mpsc::Receiver;
use std::thread;
use std::time::Instant;

use log::Logger;
use memory::RingBuffer;
use vm::{AudioBlock, Command};

pub struct Debug;

impl Debug {
    pub fn new(_: RingBuffer<AudioBlock>,
               logger: Logger,
               channel: Receiver<Command>)
               -> Self {
        thread::spawn(move || {
            let start = Instant::now();
            while let Ok(msg) = channel.recv() {
                let time = Instant::now() - start;
                logger.log_cmd(time, "backend", &msg);
            }
        });
        Debug {}
    }
}
