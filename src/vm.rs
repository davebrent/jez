use std::convert::From;
use std::sync::mpsc::{channel, Sender, Receiver};
use std::thread;
use std::time::{Duration, Instant};

use err::{JezErr, SysErr};
use lang::Program;
use log::Logger;
use mpu::Mpu;
use spu::Spu;
use unit::{Message, Unit};


const MPU_ID: u8 = 0;
const SPU_ID: u8 = 1;

pub struct Machine;

impl Machine {
    pub fn run(backend: Sender<Message>,
               bus_send: Sender<Message>,
               bus_recv: Receiver<Message>,
               prog: &Program,
               logger: Logger)
               -> Result<bool, JezErr> {
        let (spu, spu_recv) = channel();
        let (mpu, mpu_recv) = channel();

        let spu_thread = match Spu::new(SPU_ID,
                                        prog.section("spu"),
                                        bus_send.clone(),
                                        spu_recv) {
            Some(mut unit) => {
                Some(thread::spawn(move || {
                                       let res = Duration::new(0, 1000000);
                                       unit.run_forever(res);
                                   }))
            }
            None => None,
        };

        let mpu_thread = match Mpu::new(MPU_ID,
                                        prog.section("mpu_out_note"),
                                        prog.section("mpu_out_ctrl"),
                                        bus_send.clone(),
                                        mpu_recv) {
            Some(mut unit) => {
                Some(thread::spawn(move || {
                                       let res = Duration::new(0, 1000000);
                                       unit.run_forever(res);
                                   }))
            }
            None => None,
        };

        let start = Instant::now();
        while let Ok(msg) = bus_recv.recv() {
            logger.log(Instant::now() - start, "vm", &msg);
            match msg {
                Message::Stop => {
                    mpu.send(Message::Stop).unwrap();
                    spu.send(Message::Stop).unwrap();
                    if let Some(thread) = spu_thread {
                        thread.join().unwrap();
                    }
                    if let Some(thread) = mpu_thread {
                        thread.join().unwrap();
                    }
                    return Ok(false);
                }
                Message::Reload => {
                    mpu.send(Message::Stop).unwrap();
                    spu.send(Message::Stop).unwrap();
                    if let Some(thread) = spu_thread {
                        thread.join().unwrap();
                    }
                    if let Some(thread) = mpu_thread {
                        thread.join().unwrap();
                    }
                    return Ok(true);
                }
                Message::MidiNoteOn(chan, pitch, vel) => {
                    let msg = Message::MidiNoteOn(chan, pitch, vel);
                    if backend.send(msg).is_err() {
                        return Err(From::from(SysErr::UnreachableBackend));
                    }
                }
                Message::MidiNoteOff(chan, pitch) => {
                    let msg = Message::MidiNoteOff(chan, pitch);
                    if backend.send(msg).is_err() {
                        return Err(From::from(SysErr::UnreachableBackend));
                    }
                }
                Message::MidiCtl(chan, ctl, val) => {
                    let msg = Message::MidiCtl(chan, ctl, val);
                    if backend.send(msg).is_err() {
                        return Err(From::from(SysErr::UnreachableBackend));
                    }
                }
                Message::SeqEvent(event) => {
                    mpu.send(Message::SeqEvent(event)).unwrap();
                }
                Message::Error(unit, err) => {
                    println!("Unit {} has crashed {}", unit, err);
                    return Err(err);
                }
            }
        }

        Ok(false)
    }
}
