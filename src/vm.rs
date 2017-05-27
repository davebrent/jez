//! # Virtual machine
//!
//! The virtual machine is the composition of various functional units. It
//! serves as the central hub for all unit messages and is responsible for
//! dispatching actions to other units based on those messages.

use std::sync::mpsc::{channel, Sender, Receiver};
use std::thread;

use lang::Program;
use mpu::Mpu;
use spu::Spu;
use unit::{Message, RuntimeErr};


const MPU_ID: u8 = 0;
const SPU_ID: u8 = 1;

pub struct Machine {
    bus: Receiver<Message>,
    mpu: Sender<Message>,
    spu: Sender<Message>,
    backend: Sender<Message>,
}

impl Machine {
    pub fn new(backend: Sender<Message>,
               prog: &Program)
               -> Result<Self, RuntimeErr> {
        let (bus_send, bus_recv) = channel();
        let (spu_send, spu_recv) = channel();
        let (mpu_send, mpu_recv) = channel();

        Machine::new_mpu(prog, bus_send.clone(), mpu_recv);
        Machine::new_spu(prog, bus_send.clone(), spu_recv);

        Ok(Machine {
               bus: bus_recv,
               mpu: mpu_send,
               spu: spu_send,
               backend: backend,
           })
    }

    fn new_spu(prog: &Program,
               sender: Sender<Message>,
               receiver: Receiver<Message>) {
        if let Some(mut spu) = Spu::new(SPU_ID,
                                        prog.section("spu"),
                                        sender,
                                        receiver) {
            thread::spawn(move || { spu.run_forever(); });
        }
    }

    fn new_mpu(prog: &Program,
               sender: Sender<Message>,
               receiver: Receiver<Message>) {
        if let Some(mut mpu) = Mpu::new(MPU_ID,
                                        prog.section("mpu_out"),
                                        sender,
                                        receiver) {
            thread::spawn(move || { mpu.run_forever(); });
        }
    }

    pub fn run_forever(&self) -> Result<(), RuntimeErr> {
        while let Ok(msg) = self.bus.recv() {
            match msg {
                Message::MidiNoteOn(chan, pitch, vel) => {
                    let msg = Message::MidiNoteOn(chan, pitch, vel);
                    if self.backend.send(msg).is_err() {
                        return Err(RuntimeErr::BackendUnreachable);
                    }
                }
                Message::MidiNoteOff(chan, pitch) => {
                    let msg = Message::MidiNoteOff(chan, pitch);
                    if self.backend.send(msg).is_err() {
                        return Err(RuntimeErr::BackendUnreachable);
                    }
                }
                Message::MidiCtl(chan, ctl, val) => {
                    let msg = Message::MidiCtl(chan, ctl, val);
                    if self.backend.send(msg).is_err() {
                        return Err(RuntimeErr::BackendUnreachable);
                    }
                }
                Message::SeqEvent(event) => {
                    self.mpu.send(Message::SeqEvent(event)).unwrap();
                }
                Message::HasError(unit, err) => {
                    println!("Unit {} has crashed {}", unit, err);
                    return Err(err);
                }
                _ => {}
            }
        }
        Ok(())
    }
}

impl Drop for Machine {
    fn drop(&mut self) {
        self.mpu.send(Message::Stop).unwrap();
        self.spu.send(Message::Stop).unwrap();
    }
}
