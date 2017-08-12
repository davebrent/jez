use serde_json;

use math::dur_to_millis;

use std::ffi::OsString;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{Sender, Receiver};
use std::thread;
use std::time::Duration;
use vm::Command;


#[derive(Clone, Debug, Serialize)]
pub struct LogMessage {
    pub time: Duration,
    pub tag: &'static str,
    pub data: Command,
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

    pub fn log(&self, time: Duration, tag: &'static str, msg: &Command) {
        let msg = LogMessage {
            time: time,
            tag: tag,
            data: *msg,
        };
        self.channel.send(msg).unwrap();
    }
}
