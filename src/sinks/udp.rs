use std::net::UdpSocket;

use crate::err::Error;
use crate::vm::Command;

use super::osc::encode;
use super::sink::Sink;

pub struct Udp {
    sock: UdpSocket,
}

impl Udp {
    pub fn new(host_addr: &str, client_addr: &str) -> Result<Self, Error> {
        let sock = UdpSocket::bind(host_addr)?;
        sock.connect(client_addr)?;
        Ok(Udp { sock: sock })
    }
}

impl Sink for Udp {
    fn name(&self) -> &str {
        "udp"
    }

    fn process(&mut self, cmd: Command) {
        if let Some(buff) = encode(cmd) {
            self.sock.send(&buff).unwrap();
        }
    }
}
