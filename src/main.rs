extern crate docopt;
extern crate jez;
extern crate rustc_serialize;

use std::convert::From;
use std::fs;
use std::io;
use std::io::{Read, Write};
use std::sync::mpsc::{channel, Sender};
use std::thread;
use std::time::{Duration, Instant};

use docopt::Docopt;

use jez::{Instr, JezErr, Logger, Machine, make_program, make_log_backend,
          make_vm_backend, Message, millis_to_dur, RuntimeErr};


const USAGE: &'static str = "
Jez.

Usage:
  jez [options] <file>
  jez (-h | --help)
  jez --version

Options:
  -h --help         Show this screen.
  --watch           Reload input file on changes.
  --time=MS         Length of time (in milliseconds) to run for.
  --backend=NAME    Specify the backend (either 'jack' OR 'debug').
  --logger=NAME     Logging backend (either 'console' OR 'file').
";

#[derive(Debug, RustcDecodable)]
struct Args {
    flag_backend: String,
    flag_logger: String,
    flag_time: String,
    flag_watch: bool,
    flag_version: bool,
    arg_file: String,
}

fn start_timer(millis: f64, channel: Sender<Message>) {
    let start = Instant::now();
    let end = millis_to_dur(millis);
    let res = Duration::new(0, 1000000);

    thread::spawn(move || loop {
                      if start.elapsed() >= end {
                          channel.send(Message::Stop).unwrap();
                          return;
                      }
                      thread::sleep(res);
                  });
}

fn watch_file(filepath: String, instrs: &[Instr], channel: Sender<Message>) {
    let instrs = instrs.to_vec();
    thread::spawn(move || {
        let dur = Duration::new(1, 0);
        let meta_data = fs::metadata(filepath.clone()).unwrap();
        let mod_time = meta_data.modified().expect("File has been deleted");

        loop {
            let new_meta_data = fs::metadata(filepath.clone()).unwrap();
            let new_mod_time =
                new_meta_data.modified().expect("File has been deleted");

            if new_mod_time != mod_time {
                if let Ok(mut fp) = fs::File::open(filepath.clone()) {
                    let mut txt = String::new();
                    if fp.read_to_string(&mut txt).is_ok() {
                        if let Ok(next) = make_program(txt.as_str()) {
                            if instrs != next {
                                channel.send(Message::Reload).unwrap();
                                return;
                            }
                        }
                    }
                }
            }

            thread::sleep(dur);
        }
    });
}

fn run_app(args: &Args) -> Result<(), JezErr> {
    let (log_send, log_recv) = channel();
    let log_backend = try!(make_log_backend(args.flag_logger.as_ref()));
    log_backend.run_forever(log_recv);

    let (audio_send, audio_recv) = channel();
    let mut backend = try!(make_vm_backend(args.flag_backend.as_ref(),
                                           Logger::new(log_send.clone()),
                                           audio_recv));

    loop {
        let mut txt = String::new();
        let mut fp = try!(fs::File::open(args.arg_file.clone()));
        try!(fp.read_to_string(&mut txt));

        let instrs = try!(make_program(txt.as_str()));
        let (host_send, host_recv) = channel();
        if args.flag_watch {
            watch_file(args.arg_file.clone(), &instrs, host_send.clone());
        }

        if !args.flag_time.is_empty() {
            match args.flag_time.parse::<f64>() {
                Ok(time) => start_timer(time, host_send.clone()),
                Err(_) => return Err(From::from(RuntimeErr::InvalidArgs)),
            }
        }

        let res = Machine::realtime(audio_send.clone(),
                                    host_send.clone(),
                                    host_recv,
                                    &instrs,
                                    Logger::new(log_send.clone()));
        match res {
            Ok(reload) => {
                if !reload {
                    return Ok(());
                }
                backend.drain();
            }
            Err(err) => {
                return Err(err);
            }
        }
    }
}

fn main() {
    let args: Args = Docopt::new(USAGE)
        .and_then(|d| d.decode())
        .unwrap_or_else(|e| e.exit());
    if args.flag_version {
        println!("v0.3.0");
        return;
    }

    let code = match run_app(&args) {
        Ok(_) => 0,
        Err(err) => {
            writeln!(io::stderr(), "Error: {}", err).unwrap();
            1
        }
    };

    std::process::exit(code);
}
