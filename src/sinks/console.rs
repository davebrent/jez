use std::sync::mpsc::Receiver;
use std::thread;

use vm::{AudioBlock, Command, RingBuffer};

pub struct Console;

impl Console {
    pub fn new(_: RingBuffer<AudioBlock>, channel: Receiver<Command>) -> Self {
        thread::spawn(move || while let Ok(cmd) = channel.recv() {
            println!("{:?}", cmd);
        });
        Console {}
    }
}
