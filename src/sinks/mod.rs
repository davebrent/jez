mod console;
mod osc;
#[cfg(feature = "with-portmidi")]
mod portmidi;
mod sink;
#[cfg(feature = "with-websocket")]
mod ws;

use err::Error;

pub use self::console::Console;
pub use self::osc::Osc;
#[cfg(feature = "with-portmidi")]
pub use self::portmidi::Portmidi;
use self::sink::{CompositeSink, Sink, ThreadedSink};
#[cfg(feature = "with-websocket")]
pub use self::ws::WebSocket;

#[derive(Clone, Debug, PartialEq)]
pub struct SinkArgs<'a> {
    osc_host_addr: &'a str,
    osc_client_addr: &'a str,
    midi_device_id: Option<usize>,
    ws_host_addr: &'a str,
}

impl<'a> SinkArgs<'a> {
    pub fn new(
        osc_host_addr: &'a str,
        osc_client_addr: &'a str,
        ws_host_addr: &'a str,
        midi_device_id: Option<usize>,
    ) -> SinkArgs<'a> {
        SinkArgs {
            osc_host_addr: osc_host_addr,
            osc_client_addr: osc_client_addr,
            ws_host_addr: ws_host_addr,
            midi_device_id: midi_device_id,
        }
    }
}

pub fn factory(name: &str, args: &SinkArgs) -> Result<Box<Sink>, Error> {
    let sink: Box<Sink> = match name {
        "console" | "" => Box::new(Console::new()),
        "osc" => Box::new(try!(Osc::new(args.osc_host_addr, args.osc_client_addr))),
        #[cfg(feature = "with-portmidi")]
        "portmidi" => Box::new(try!(Portmidi::new(args.midi_device_id))),
        #[cfg(feature = "with-websocket")]
        "websocket" => Box::new(try!(WebSocket::new(args.ws_host_addr))),
        _ => return Err(error!(UnknownBackend, name)),
    };

    Ok(sink)
}

pub fn make_sink(names: &str, args: &SinkArgs) -> Result<Box<Sink>, Error> {
    let mut sinks = vec![];
    for name in names.split(',') {
        let sink = try!(factory(name, args));
        sinks.push(sink);
    }

    let comp = Box::new(CompositeSink::new(sinks));
    Ok(Box::new(ThreadedSink::new(comp)))
}
