use vm::Command;

use super::sink::Sink;

pub struct Console;

impl Console {
    pub fn new() -> Self {
        Console {}
    }
}

impl Sink for Console {
    fn name(&self) -> &str {
        "console"
    }

    fn recieve(&mut self, cmd: Command) {
        println!("{:?}", cmd);
    }
}
