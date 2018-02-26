use std::net::UdpSocket;

use err::Error;
use vm::Command;

use super::sink::Sink;
use super::osc::encode;

pub struct Udp {
    sock: UdpSocket,
}

impl Udp {
    pub fn new(host_addr: &str, client_addr: &str) -> Result<Self, Error> {
        let sock = try!(UdpSocket::bind(host_addr));
        try!(sock.connect(client_addr));
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
