use std::sync::mpsc::Receiver;
use std::thread;

use vm::Command;

pub struct Console;

impl Console {
    pub fn new(channel: Receiver<Command>) -> Self {
        thread::spawn(move || while let Ok(cmd) = channel.recv() {
            println!("{:?}", cmd);
        });
        Console {}
    }
}
