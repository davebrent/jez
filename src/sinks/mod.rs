mod console;
mod osc;
#[cfg(feature = "with-portmidi")]
mod portmidi;
mod sink;
mod udp;
#[cfg(feature = "with-websocket")]
mod ws;

use crate::err::Error;

pub use self::sink::{CompositeSink, Device, Sink, ThreadedSink};

#[derive(Clone, Debug, PartialEq)]
pub enum Backend<'a> {
    Console,
    PortMidi(Option<usize>),
    Udp(&'a str, &'a str),
    WebSocket(&'a str),
}

pub fn factory(request: &Backend) -> Result<Box<dyn Sink>, Error> {
    #[allow(unreachable_patterns)]
    Ok(match *request {
        Backend::Console => Box::new(console::Console::new()),
        Backend::Udp(host, client) => Box::new(r#try!(udp::Udp::new(host, client))),
        #[cfg(feature = "with-websocket")]
        Backend::WebSocket(host) => Box::new(r#try!(ws::WebSocket::new(host))),
        #[cfg(feature = "with-portmidi")]
        Backend::PortMidi(device) => Box::new(r#try!(portmidi::Portmidi::new(device))),
        _ => return Err(error!(UnknownBackend, &format!("{:?}", request))),
    })
}
