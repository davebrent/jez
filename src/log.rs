use serde_json;

use math::dur_to_millis;

use std::ffi::OsString;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{Receiver, Sender};
use std::thread;
use std::time::Duration;
use vm::{Command, Event};

#[derive(Clone, Debug, Serialize)]
pub enum LogData {
    Event(Event),
    Command(Command),
}

#[derive(Clone, Debug, Serialize)]
pub struct LogMessage {
    pub time: Duration,
    pub tag: &'static str,
    pub data: LogData,
}

pub trait LogBackend {
    fn run_forever(&self, channel: Receiver<LogMessage>);
}

#[derive(Debug)]
pub struct ConsoleLogger;

impl ConsoleLogger {
    pub fn new() -> ConsoleLogger {
        ConsoleLogger {}
    }
}

impl LogBackend for ConsoleLogger {
    fn run_forever(&self, channel: Receiver<LogMessage>) {
        thread::spawn(move || while let Ok(msg) = channel.recv() {
            let millis = dur_to_millis(&msg.time);
            println!("{}, {}, {:?}", millis, msg.tag, msg.data);
        });
    }
}

#[derive(Debug)]
pub struct FileLogger;

impl FileLogger {
    pub fn new() -> FileLogger {
        FileLogger {}
    }
}

fn unique_filename(pattern: &str) -> OsString {
    let mut i = 1;
    let mut buff = PathBuf::from(pattern);

    loop {
        let tbuf = buff.clone();
        let filepath = Path::new(&tbuf);
        if !filepath.exists() {
            return buff.into_os_string();
        }

        let orig = Path::new(pattern);
        let stem = orig.file_stem().unwrap().to_str().unwrap();
        let ext = orig.extension().unwrap().to_str().unwrap();
        buff.set_file_name(format!("{}-{}.{}", stem, i, ext));
        i += 1;
    }
}

impl LogBackend for FileLogger {
    fn run_forever(&self, channel: Receiver<LogMessage>) {
        let mut file = fs::File::create(unique_filename("jez.log")).unwrap();
        thread::spawn(move || while let Ok(msg) = channel.recv() {
            let s = serde_json::to_string(&msg).unwrap() + "\n";
            file.write_all(s.as_bytes()).unwrap();
        });
    }
}

#[derive(Debug)]
pub struct Logger {
    channel: Sender<LogMessage>,
}

impl Logger {
    pub fn new(channel: Sender<LogMessage>) -> Logger {
        Logger { channel: channel }
    }

    pub fn log_event(&self, time: Duration, tag: &'static str, evt: &Event) {
        let msg = LogMessage {
            time: time,
            tag: tag,
            data: LogData::Event(*evt),
        };
        self.channel.send(msg).ok();
    }

    pub fn log_cmd(&self, time: Duration, tag: &'static str, cmd: &Command) {
        let msg = LogMessage {
            time: time,
            tag: tag,
            data: LogData::Command(*cmd),
        };
        self.channel.send(msg).ok();
    }
}
