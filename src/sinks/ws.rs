use std::sync::mpsc::{channel, Receiver, Sender};
use std::thread;

use ws;

use err::SysErr;
use vm::Command;

use super::osc::Osc;
use super::sink::Sink;

impl From<ws::Error> for SysErr {
    fn from(_: ws::Error) -> SysErr {
        SysErr::UnreachableBackend
    }
}

enum WebSocketEvent {
    Connected(usize, ws::Sender),
    Disconnected(usize),
}

struct WebSocketHandler {
    id: usize,
    out: ws::Sender,
    channel: Sender<WebSocketEvent>,
}

struct WebSocketServer {
    channel: Sender<WebSocketEvent>,
}

pub struct WebSocket {
    channel: Receiver<WebSocketEvent>,
    clients: Vec<(usize, ws::Sender)>,
    _incoming: thread::JoinHandle<Result<(), SysErr>>,
}

impl ws::Handler for WebSocketHandler {
    fn on_open(&mut self, _: ws::Handshake) -> ws::Result<()> {
        let cmd = WebSocketEvent::Connected(self.id, self.out.clone());
        self.channel.send(cmd).ok();
        Ok(())
    }

    fn on_close(&mut self, _: ws::CloseCode, _: &str) {
        let cmd = WebSocketEvent::Disconnected(self.id);
        self.channel.send(cmd).ok();
    }
}

impl WebSocketServer {
    pub fn new(channel: Sender<WebSocketEvent>) -> Result<WebSocketServer, SysErr> {
        Ok(WebSocketServer { channel: channel })
    }

    pub fn run_forever(&mut self, host_addr: &str) -> Result<(), SysErr> {
        let mut ids = 0;
        try!(ws::listen(host_addr, |out| {
            ids += 1;
            WebSocketHandler {
                id: ids,
                out: out,
                channel: self.channel.clone(),
            }
        }));
        Ok(())
    }
}

impl WebSocket {
    pub fn new(host_addr: &str) -> Result<Self, SysErr> {
        let (tx, rx) = channel();

        let mut server = try!(WebSocketServer::new(tx));
        let host_addr = host_addr.to_string();
        let incoming = thread::spawn(move || server.run_forever(&host_addr));

        Ok(WebSocket {
            channel: rx,
            clients: vec![],
            _incoming: incoming,
        })
    }
}

impl Sink for WebSocket {
    fn name(&self) -> &str {
        "websocket"
    }

    fn recieve(&mut self, cmd: Command) {
        while let Ok(event) = self.channel.try_recv() {
            match event {
                WebSocketEvent::Connected(id, client) => {
                    self.clients.push((id, client));
                }
                WebSocketEvent::Disconnected(id) => {
                    self.clients.retain(|&(cid, _)| cid != id);
                }
            }
        }

        if let Some(data) = Osc::encode(cmd) {
            for &(_, ref client) in &self.clients {
                client.send(data.clone()).ok();
            }
        }
    }
}
