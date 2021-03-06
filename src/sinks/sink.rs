use std::fmt;
use std::sync::mpsc::Receiver;
use std::thread;

use crate::vm::Command;

pub trait Device: fmt::Display {}

pub trait Sink: Send {
    fn name(&self) -> &str;

    fn process(&mut self, cmd: Command);

    fn devices(&self) -> Vec<Box<dyn Device>> {
        vec![]
    }

    fn run_forever(&mut self, channel: Receiver<Command>) {
        while let Ok(msg) = channel.recv() {
            self.process(msg);
        }
    }
}

pub struct CompositeSink {
    inner: Vec<Box<dyn Sink>>,
    name: String,
}

impl CompositeSink {
    pub fn new(sinks: Vec<Box<dyn Sink>>) -> CompositeSink {
        let name = sinks
            .iter()
            .map(|s| s.name())
            .collect::<Vec<_>>()
            .join(", ");

        CompositeSink {
            inner: sinks,
            name: name,
        }
    }
}

impl Sink for CompositeSink {
    fn name(&self) -> &str {
        &self.name
    }

    fn devices(&self) -> Vec<Box<dyn Device>> {
        let mut devices = vec![];
        for sink in &self.inner {
            let mut devs = sink.devices();
            devices.append(&mut devs);
        }
        devices
    }

    fn process(&mut self, cmd: Command) {
        for sink in &mut self.inner {
            sink.process(cmd);
        }
    }
}

pub struct ThreadedSink {
    inner: Option<Box<dyn Sink>>,
}

impl ThreadedSink {
    pub fn new(sink: Box<dyn Sink>) -> ThreadedSink {
        ThreadedSink { inner: Some(sink) }
    }
}

impl Sink for ThreadedSink {
    fn name(&self) -> &str {
        match self.inner {
            Some(ref sink) => sink.name(),
            None => "",
        }
    }

    fn devices(&self) -> Vec<Box<dyn Device>> {
        match self.inner {
            Some(ref sink) => sink.devices(),
            None => vec![],
        }
    }

    fn run_forever(&mut self, channel: Receiver<Command>) {
        let mut sink = match self.inner.take() {
            Some(sink) => sink,
            None => return,
        };
        thread::spawn(move || {
            while let Ok(cmd) = channel.recv() {
                sink.process(cmd);
            }
        });
    }

    fn process(&mut self, cmd: Command) {
        if let Some(ref mut sink) = self.inner {
            sink.process(cmd);
        }
    }
}
