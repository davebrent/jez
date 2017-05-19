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
use unit::Message;


const MPU_ID: u8 = 0;
const SPU_ID: u8 = 1;

pub struct Machine {
    glob: Receiver<Message>,
    mpu: Sender<Message>,
    spu: Sender<Message>,
}

impl Machine {
    pub fn new(prog: &Program) -> Machine {
        let (main_send, main_recv) = channel();

        let (spu_send, spu_recv) = channel();
        let mut spu =
            Spu::new(SPU_ID, prog.section("spu"), main_send.clone(), spu_recv)
                .unwrap();

        let (mpu_send, mpu_recv) = channel();
        let mut mpu = Mpu::new(MPU_ID,
                               prog.section("mpu_out"),
                               main_send.clone(),
                               mpu_recv)
                .unwrap();

        thread::spawn(move || { spu.run_forever(); });
        thread::spawn(move || { mpu.run_forever(); });

        Machine {
            glob: main_recv,
            mpu: mpu_send,
            spu: spu_send,
        }
    }

    pub fn tick(&self) {
        while let Ok(msg) = self.glob.try_recv() {
            match msg {
                Message::TriggerEvent(val, dur) => {
                    self.mpu.send(Message::TriggerEvent(val, dur)).unwrap();
                }
                Message::HasError(unit, err) => {
                    println!("Unit {} has crashed {}", unit, err);
                }
                _ => {}
            }
        }
    }
}

impl Drop for Machine {
    fn drop(&mut self) {
        self.mpu.send(Message::Stop).unwrap();
        self.spu.send(Message::Stop).unwrap();
    }
}
