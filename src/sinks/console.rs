use std::sync::mpsc::Receiver;
use std::thread;

use memory::RingBuffer;
use vm::{AudioBlock, Command};

pub struct Console;

impl Console {
    pub fn new(_: RingBuffer<AudioBlock>, channel: Receiver<Command>) -> Self {
        thread::spawn(move || while let Ok(cmd) = channel.recv() {
            println!("{:?}", cmd);
        });
        Console {}
    }
}
