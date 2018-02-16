mod err;
mod lang;
mod sinks;
mod vm;

extern crate byteorder;
#[cfg(feature = "with-portmidi")]
extern crate portmidi;
extern crate rand;
extern crate rosc;
extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate serde_json;
#[cfg(feature = "with-websocket")]
extern crate ws;

use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_double};
use std::str;

use std::mem;
use std::sync::mpsc::channel;
use std::time::Duration;

pub use err::JezErr;
pub use err::RuntimeErr;
pub use sinks::{make_sink, SinkArgs};
pub use vm::{millis_to_dur, Command, Control, Destination, Event, EventValue, Instr, InterpState,
             Machine, Value};

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

pub fn eval(rev: usize, func: &str, prog: &str) -> Result<(Value, InterpState), JezErr> {
    let (back_send, _) = channel();
    let (host_send, host_recv) = channel();

    let instrs = try!(make_program(prog));
    let mut machine = Machine::new(back_send, host_send, host_recv, &instrs);

    let value = try!(machine.eval(func, rev));
    Ok((value, machine.interp.state))
}

pub fn simulate(dur: Duration, dt: Duration, prog: &str) -> Result<Simulation, JezErr> {
    let (back_send, back_recv) = channel();
    let (host_send, host_recv) = channel();

    let instrs = try!(make_program(prog));
    let mut machine = Machine::new(back_send, host_send, host_recv, &instrs);

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
pub extern "C" fn jez_simulate(dur: c_double, dt: c_double, prog: *const c_char) -> *const c_char {
    let dur = millis_to_dur(dur);
    let dt = millis_to_dur(dt);
    let prog = to_str(prog);
    let out = simulate(dur, dt, prog);
    let out = CString::new(serde_json::to_string(&out).unwrap()).unwrap();
    let ptr = out.as_ptr();
    mem::forget(out);
    ptr
}

#[no_mangle]
pub extern "C" fn jez_eval(rev: usize, func: *const c_char, prog: *const c_char) -> *const c_char {
    let func = to_str(func);
    let prog = to_str(prog);
    let out = eval(rev, func, prog);
    let out = CString::new(serde_json::to_string(&out).unwrap()).unwrap();
    let ptr = out.as_ptr();
    mem::forget(out);
    ptr
}
