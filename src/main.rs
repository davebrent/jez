extern crate docopt;
extern crate jez;
extern crate rustc_serialize;

use std::convert::From;
use std::fs;
use std::io;
use std::io::{Read, Write};
use std::sync::mpsc::{Sender, channel};
use std::thread;
use std::time::{Duration, Instant};

use docopt::Docopt;

use jez::{AudioBlock, Command, Control, Instr, JezErr, Logger, Machine,
          RingBuffer, RuntimeErr, make_log_backend, make_program,
          make_vm_backend, millis_to_dur};

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
  --backend=NAME    Specify the backend (either 'debug', 'jack' OR 'portaudio').
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

fn start_timer(millis: f64, channel: Sender<Command>) {
    let start = Instant::now();
    let end = millis_to_dur(millis);
    let res = Duration::new(0, 1_000_000);

    thread::spawn(move || loop {
        if start.elapsed() >= end {
            channel.send(Command::Stop).unwrap();
            return;
        }
        thread::sleep(res);
    });
}

fn watch_file(filepath: String, instrs: &[Instr], channel: Sender<Command>) {
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
                                channel.send(Command::Reload).unwrap();
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
    let ring = RingBuffer::new(64, AudioBlock::new(64));

    let (log_send, log_recv) = channel();
    let log_backend = try!(make_log_backend(args.flag_logger.as_ref()));
    log_backend.run_forever(log_recv);

    let (audio_send, audio_recv) = channel();
    let mut _backend = try!(make_vm_backend(
        args.flag_backend.as_ref(),
        ring.clone(),
        Logger::new(log_send.clone()),
        audio_recv,
    ));

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

        let mut machine = Machine::new(
            ring.clone(),
            audio_send.clone(),
            host_send.clone(),
            host_recv,
            &instrs,
            Logger::new(log_send.clone()),
        );

        match try!(machine.exec_realtime()) {
            Control::Stop => return Ok(()),
            _ => continue,
        }
    }
}

fn main() {
    let args: Args = Docopt::new(USAGE)
        .and_then(|d| d.decode())
        .unwrap_or_else(|e| e.exit());
    if args.flag_version {
        println!("v0.4.0");
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
