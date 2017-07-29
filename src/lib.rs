mod assem;
mod backends;
mod err;
mod interp;
mod log;
mod math;
mod mpu;
mod parse;
mod spu;
mod unit;
mod vm;

extern crate docopt;
#[cfg(feature = "with-jack")]
extern crate jack;
extern crate libc;
#[macro_use]
extern crate nom;
extern crate rand;
extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate serde_json;

use libc::{c_char, c_double};
use std::ffi::{CStr, CString};
use std::str;

use std::convert::From;
use std::mem;
use std::sync::mpsc::{channel, Receiver};
use std::time::Duration;

pub use unit::Message;
pub use interp::Instr;
pub use err::JezErr;
pub use err::RuntimeErr;
pub use vm::Machine;
pub use log::Logger;
pub use math::millis_to_dur;


pub fn make_vm_backend(name: &str,
                       logger: log::Logger,
                       channel: Receiver<unit::Message>)
                       -> Result<Box<backends::Backend>, err::JezErr> {
    match name {
        "debug" | "" => Ok(Box::new(backends::Debug::new(logger, channel))),
        #[cfg(feature = "with-jack")]
        "jack" => Ok(Box::new(try!(backends::Jack::new(logger, channel)))),
        _ => Err(From::from(err::SysErr::UnknownBackend)),
    }
}

pub fn make_log_backend(name: &str)
                        -> Result<Box<log::LogBackend>, err::JezErr> {
    match name {
        "console" | "" => Ok(Box::new(log::ConsoleLogger::new())),
        "file" => Ok(Box::new(log::FileLogger::new())),
        _ => Err(From::from(err::SysErr::UnknownBackend)),
    }
}

pub fn make_program(txt: &str) -> Result<Vec<interp::Instr>, err::JezErr> {
    let dirs = try!(parse::parser(txt));
    let instrs = try!(assem::assemble(&dirs));
    Ok(instrs)
}

#[derive(Debug, Serialize)]
struct Simulation {
    duration: Duration,
    delta: Duration,
    instructions: Vec<Instr>,
    messages: Vec<log::LogMessage>,
}

fn simulate(dur: Duration,
            dt: Duration,
            prog: &str)
            -> Result<Simulation, JezErr> {
    let (log_send, log_recv) = channel();
    let (audio_send, _audio_recv) = channel();
    let (host_send, host_recv) = channel();

    let instrs = try!(make_program(prog));
    try!(Machine::simulate(dur,
                           dt,
                           audio_send.clone(),
                           host_send.clone(),
                           host_recv,
                           &instrs,
                           Logger::new(log_send.clone())));

    let mut msgs = Vec::new();
    while let Ok(msg) = log_recv.try_recv() {
        msgs.push(msg);
    }

    Ok(Simulation {
           duration: dur,
           delta: dt,
           instructions: instrs,
           messages: msgs,
       })
}

fn to_str<'a>(s: *const c_char) -> &'a str {
    if s.is_null() {
        return "";
    }
    let c_str = unsafe { CStr::from_ptr(s) };
    c_str.to_str().unwrap()
}

#[no_mangle]
pub extern "C" fn jez_simulate(dur: c_double,
                               dt: c_double,
                               prog: *const c_char)
                               -> *const c_char {
    let dur = millis_to_dur(dur);
    let dt = millis_to_dur(dt);
    let prog = to_str(prog);
    let out = simulate(dur, dt, prog);
    let out = CString::new(serde_json::to_string(&out).unwrap()).unwrap();
    let ptr = out.as_ptr();
    mem::forget(out);
    ptr
}
