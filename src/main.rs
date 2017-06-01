mod backends;
mod err;
mod lang;
mod math;
mod mpu;
mod spu;
mod unit;
mod vm;

extern crate docopt;
extern crate jack;
extern crate rand;
extern crate regex;
extern crate rustc_serialize;

use std::convert::From;
use std::fs;
use std::io;
use std::io::{Read, Write};
use std::sync::mpsc::{channel, Sender, Receiver};
use std::thread;
use std::time::Duration;

use docopt::Docopt;


const USAGE: &'static str = "
Jez.

Usage:
  jez [options] <file>
  jez (-h | --help)
  jez --version

Options:
  -h --help         Show this screen.
  --watch           Reload input file on changes.
  --backend=NAME    Specify the backend (either 'jack' OR 'debug').
";

#[derive(Debug, RustcDecodable)]
struct Args {
    flag_backend: String,
    flag_watch: bool,
    flag_version: bool,
    arg_file: String,
}

fn watch_file(filepath: String,
              prog: lang::Program,
              channel: Sender<unit::Message>) {
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
                        if let Ok(next) = lang::Program::new(txt.as_str()) {
                            if prog != next {
                                channel.send(unit::Message::Reload).unwrap();
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

fn make_backend(name: &str,
                channel: Receiver<unit::Message>)
                -> Result<Box<backends::Backend>, err::JezErr> {
    match name {
        "debug" | "" => Ok(Box::new(backends::Debug::new(channel))),
        "jack" => Ok(Box::new(try!(backends::Jack::new(channel)))),
        _ => Err(From::from(err::SysErr::UnknownBackend)),
    }
}

fn run_app(args: &Args) -> Result<(), err::JezErr> {
    let (audio_send, audio_recv) = channel();
    let mut backend = try!(make_backend(args.flag_backend.as_ref(),
                                        audio_recv));

    loop {
        let mut txt = String::new();
        let mut fp = try!(fs::File::open(args.arg_file.clone()));
        try!(fp.read_to_string(&mut txt));

        let prog = try!(lang::Program::new(txt.as_str()));
        let (host_send, host_recv) = channel();
        if args.flag_watch {
            watch_file(args.arg_file.clone(), prog.clone(), host_send.clone());
        }

        let res = vm::Machine::run(audio_send.clone(),
                                   host_send.clone(),
                                   host_recv,
                                   &prog);
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
        println!("v0.1.0");
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
