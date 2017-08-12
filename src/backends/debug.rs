use std::sync::mpsc::Receiver;
use std::thread;
use std::time::Instant;

use log::Logger;
use vm::Command;


pub struct Debug;

impl Debug {
    pub fn new(logger: Logger, channel: Receiver<Command>) -> Self {
        thread::spawn(move || {
                          let start = Instant::now();
                          while let Ok(msg) = channel.recv() {
                              let time = Instant::now() - start;
                              logger.log(time, "backend", &msg);
                          }
                      });
        Debug {}
    }
}
