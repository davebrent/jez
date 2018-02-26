#[macro_use]
mod err;
mod api;
mod capi;
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

pub use api::{simulate, Machine, Program, Sink};
pub use capi::jez_simulate;
pub use err::{Error, Kind, Location};
pub use sinks::{Backend, Device};
pub use vm::{Command, Status};
