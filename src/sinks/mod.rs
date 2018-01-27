mod console;
mod osc;
#[cfg(feature = "with-portmidi")]
mod portmidi;

use std::any::Any;
use std::convert::From;
use std::sync::mpsc::Receiver;

use err::{JezErr, SysErr};
use vm::Command;

pub use self::console::Console;
pub use self::osc::Osc;
#[cfg(feature = "with-portmidi")]
pub use self::portmidi::Portmidi;


pub fn make_sink(name: &str,
                 channel: Receiver<Command>)
                 -> Result<Box<Any>, JezErr> {
    match name {
        "console" | "" => Ok(Box::new(Console::new(channel))),
        "osc" => Ok(Box::new(try!(Osc::new(channel)))),
        #[cfg(feature = "with-portmidi")]
        "portmidi" => Ok(Box::new(try!(Portmidi::new(channel)))),
        _ => Err(From::from(SysErr::UnknownBackend)),
    }
}
