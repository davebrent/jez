mod console;
#[cfg(feature = "with-jack")]
mod jack;
mod osc;
#[cfg(feature = "with-portaudio")]
mod portaudio;

use std::any::Any;
use std::convert::From;
use std::sync::mpsc::Receiver;

use err::{JezErr, SysErr};
use memory::RingBuffer;
use vm::{AudioBlock, Command};

pub use self::console::Console;
#[cfg(feature = "with-jack")]
pub use self::jack::Jack;
pub use self::osc::Osc;
#[cfg(feature = "with-portaudio")]
pub use self::portaudio::Portaudio;

pub fn make_sink(name: &str,
                 rb: RingBuffer<AudioBlock>,
                 channel: Receiver<Command>)
                 -> Result<Box<Any>, JezErr> {
    match name {
        "console" | "" => Ok(Box::new(Console::new(rb, channel))),
        #[cfg(feature = "with-jack")]
        "jack" => Ok(Box::new(try!(Jack::new(rb, channel)))),
        "osc" => Ok(Box::new(try!(Osc::new(rb, channel)))),
        #[cfg(feature = "with-portaudio")]
        "portaudio" => Ok(Box::new(try!(Portaudio::new(rb, channel)))),
        _ => Err(From::from(SysErr::UnknownBackend)),
    }
}