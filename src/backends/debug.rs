use std::sync::mpsc::Receiver;
use std::thread;

use memory::RingBuffer;
use vm::{AudioBlock, Command};

pub struct Debug;

impl Debug {
    pub fn new(_: RingBuffer<AudioBlock>, channel: Receiver<Command>) -> Self {
        thread::spawn(move || while let Ok(cmd) = channel.recv() {
            println!("{:?}", cmd);
        });
        Debug {}
    }
}
