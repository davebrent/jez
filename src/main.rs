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
/// Parser for custom bytecode
mod lang;
/// MIDI processing unit
mod mpu;
/// Sequencer processing unit
mod spu;
/// Base & shared unit functionality
mod unit;
/// Virtual machine
mod vm;

extern crate clap;
extern crate jack;
extern crate jack_sys;
extern crate rand;
extern crate regex;

use std::fs::File;
use std::io;
use std::sync::mpsc::channel;

use clap::{Arg, App};


/// Return a file reader or load from stdin
fn input(fp: Option<&str>) -> Result<Box<io::Read>, std::io::Error> {
    match fp {
        Some(fp) => Ok(Box::new(try!(File::open(fp)))),
        None => Ok(Box::new(io::stdin())),
    }
}

fn run_forever(prog: lang::Program) -> Result<(), unit::RuntimeErr> {
    let (tx, rx) = channel();
    let _jck = try!(backends::Jack::new(rx));
    let mach = try!(vm::Machine::new(tx, &prog));
    return mach.run_forever();
}

fn main() {
    let matches = App::new("Jez")
        .about("An audio visual stack machine")
        .after_help("\
Jez is a stack machine & JACK client for generating musical sequences and audio
reactive visualisations.")
        .arg(Arg::with_name("file")
                 .short("f")
                 .long("file")
                 .value_name("FILE")
                 .help("Input file to execute")
                 .takes_value(true))
        .arg(Arg::with_name("v")
                 .short("v")
                 .multiple(true)
                 .help("Sets the level of verbosity"))
        .get_matches();

    match input(matches.value_of("file")) {
        Err(_) => println!("Error, File not found"),
        Ok(mut reader) => {
            let mut txt = String::new();
            reader
                .read_to_string(&mut txt)
                .expect("Unrecognised data in file");

            match lang::Program::new(txt.as_str()) {
                Err(err) => println!("Compile error, {}", err),
                Ok(prog) => {
                    match run_forever(prog) {
                        Err(err) => {
                            println!("Runtime error, {}", err);
                        }
                        _ => (),
                    }
                }
            }
        }
    }
}
