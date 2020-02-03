#[macro_use]
mod err;
mod api;
mod capi;
mod lang;
mod sinks;
mod vm;

pub use crate::api::{simulate, Machine, Program, Sink};
pub use crate::capi::jez_simulate;
pub use crate::err::{Error, Kind, Location};
pub use crate::sinks::{Backend, Device};
pub use crate::vm::{Command, Status};
