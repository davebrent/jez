mod console;
mod null;
mod osc;
#[cfg(feature = "with-portmidi")]
mod portmidi;
mod renoise;
mod sink;
mod udp;
#[cfg(feature = "with-websocket")]
mod ws;

use crate::err::Error;

pub use self::sink::{CompositeSink, Device, Sink, ThreadedSink};

#[derive(Clone, Debug, PartialEq)]
pub enum Backend<'a> {
    Console,
    Null,
    PortMidi(Option<usize>),
    Udp(&'a str, &'a str),
    Renoise(&'a str, &'a str),
    WebSocket(&'a str),
}

pub fn factory(request: &Backend) -> Result<Box<dyn Sink>, Error> {
    #[allow(unreachable_patterns)]
    Ok(match *request {
        Backend::Console => Box::new(console::Console::new()),
        Backend::Null => Box::new(null::Null::new()),
        Backend::Udp(host, client) => Box::new(udp::Udp::new(host, client)?),
        #[cfg(feature = "with-websocket")]
        Backend::WebSocket(host) => Box::new(ws::WebSocket::new(host)?),
        #[cfg(feature = "with-portmidi")]
        Backend::PortMidi(device) => Box::new(portmidi::Portmidi::new(device)?),
        Backend::Renoise(host, client) => Box::new(renoise::Renoise::new(host, client)?),
        _ => return Err(error!(UnknownBackend, &format!("{:?}", request))),
    })
}
