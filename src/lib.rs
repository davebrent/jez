mod err;
mod lang;
mod sinks;
mod vm;

extern crate byteorder;
extern crate docopt;
#[cfg(feature = "with-jack")]
extern crate jack;
extern crate libc;
#[macro_use]
extern crate nom;
#[cfg(feature = "with-portaudio")]
extern crate portaudio;
extern crate rand;
extern crate rosc;
extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate serde_json;

use libc::{c_char, c_double};
use std::ffi::{CStr, CString};
use std::str;

use std::mem;
use std::sync::mpsc::channel;
use std::time::Duration;

pub use err::JezErr;
pub use err::RuntimeErr;
pub use sinks::make_sink;
pub use vm::{AudioBlock, Command, Control, Destination, Event, EventValue,
             Instr, Machine, RingBuffer, millis_to_dur};

pub fn make_program(txt: &str) -> Result<Vec<Instr>, err::JezErr> {
    let dirs = try!(lang::parser(txt));
    let instrs = try!(lang::assemble(&dirs));
    Ok(instrs)
}

#[derive(Debug, Serialize)]
pub struct Simulation {
    pub duration: Duration,
    pub delta: Duration,
    pub instructions: Vec<Instr>,
    pub messages: Vec<Command>,
}

pub fn simulate(dur: Duration,
                dt: Duration,
                prog: &str)
                -> Result<Simulation, JezErr> {
    let ring = RingBuffer::new(64, AudioBlock::new(64));

    let (back_send, back_recv) = channel();
    let (host_send, host_recv) = channel();

    let instrs = try!(make_program(prog));
    let mut machine = Machine::new(
        ring,
        back_send.clone(),
        host_send.clone(),
        host_recv,
        &instrs,
    );

    try!(machine.exec(dur, dt));
    let mut msgs = Vec::new();
    while let Ok(msg) = back_recv.try_recv() {
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
