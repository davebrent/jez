mod console;
mod udp;
mod osc;
#[cfg(feature = "with-portmidi")]
mod portmidi;
mod sink;
#[cfg(feature = "with-websocket")]
mod ws;

use err::Error;

pub use self::sink::{CompositeSink, Device, Sink, ThreadedSink};

#[derive(Clone, Debug, PartialEq)]
pub enum Backend<'a> {
    Console,
    PortMidi(Option<usize>),
    Udp(&'a str, &'a str),
    WebSocket(&'a str),
}

pub fn factory(request: &Backend) -> Result<Box<Sink>, Error> {
    #[allow(unreachable_patterns)]
    Ok(match *request {
        Backend::Console => Box::new(console::Console::new()),
        Backend::Udp(host, client) => Box::new(try!(udp::Udp::new(host, client))),
        #[cfg(feature = "with-websocket")]
        Backend::WebSocket(host) => Box::new(try!(ws::WebSocket::new(host))),
        #[cfg(feature = "with-portmidi")]
        Backend::PortMidi(device) => Box::new(try!(portmidi::Portmidi::new(device))),
        _ => return Err(error!(UnknownBackend, &format!("{:?}", request))),
    })
}
