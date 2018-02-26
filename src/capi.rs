use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_double};
use std::str;

use std::mem;

pub use api::{simulate, Program};
pub use vm::millis_to_dur;

fn to_str<'a>(s: *const c_char) -> &'a str {
    if s.is_null() {
        return "";
    }
    let c_str = unsafe { CStr::from_ptr(s) };
    c_str.to_str().unwrap()
}

#[no_mangle]
pub extern "C" fn jez_simulate(
    duration: c_double,
    delta: c_double,
    program: *const c_char,
) -> *const c_char {
    let program = to_str(program);
    let out = simulate(duration, delta, program).unwrap();
    let out = CString::new(out).unwrap();
    let ptr = out.as_ptr();
    mem::forget(out);
    ptr
}
