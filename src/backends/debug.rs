use std::sync::mpsc::Receiver;
use std::thread;
use std::time::Instant;

use log::Logger;
use unit::Message;

use super::base::Backend;


pub struct Debug;

impl Debug {
    pub fn new(logger: Logger, channel: Receiver<Message>) -> Self {
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

impl Backend for Debug {
    fn drain(&mut self) {}
}
