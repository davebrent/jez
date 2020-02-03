use crate::vm::Command;

use super::sink::Sink;

pub struct Null;

impl Null {
    pub fn new() -> Self {
        Null {}
    }
}

impl Sink for Null {
    fn name(&self) -> &str {
        "null"
    }

    fn process(&mut self, _: Command) {}
}
