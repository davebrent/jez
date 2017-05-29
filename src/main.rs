//! # Jez
//!
//! Jez is a [stack machine][1] & [JACK][2] client for generating musical
//! sequences and audio reactive visualisations.
//!
//!   [1]: https://en.wikipedia.org/wiki/Stack_machine
//!   [2]: http://www.jackaudio.org/
//!
//! It uses a custom bytecode, written in [reverse polish notation][3], to
//! describe behaviour for sequences, visualisations or audio processing. The
//! bytecode is interpretted by a virtual machine to produce graphics, sound,
//! MIDI etc.
//!
//!   [3]: https://en.wikipedia.org/wiki/Reverse_Polish_notation
//!
//! The virtual machine is composed of 'functional units' that perform tasks
//! for specific domains communicating with other units through message passing.

mod backends;
mod err;
/// Parser for custom bytecode
mod lang;
/// Math functions
mod math;
/// MIDI processing unit
mod mpu;
/// Sequencer processing unit
mod spu;
/// Base & shared unit functionality
mod unit;
/// Virtual machine
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
    arg_file: String,
}

fn watch_file(filepath: String, channel: Sender<unit::Message>) {
    thread::spawn(move || {
        let dur = Duration::new(1, 0);
        let meta_data = fs::metadata(filepath.clone()).unwrap();
        let mod_time = meta_data.modified().expect("File has been deleted");

        loop {
            let new_meta_data = fs::metadata(filepath.clone()).unwrap();
            let new_mod_time =
                new_meta_data.modified().expect("File has been deleted");

            if new_mod_time != mod_time {
                channel.send(unit::Message::Reload).unwrap();
                return;
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
            watch_file(args.arg_file.clone(), host_send.clone());
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
    let code = match run_app(&args) {
        Ok(_) => 0,
        Err(err) => {
            writeln!(io::stderr(), "Error: {}", err).unwrap();
            1
        }
    };

    std::process::exit(code);
}
