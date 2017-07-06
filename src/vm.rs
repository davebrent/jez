use std::collections::HashMap;
use std::convert::From;
use std::sync::mpsc::{channel, Sender, Receiver};
use std::thread;
use std::time::{Duration, Instant};

use err::{JezErr, SysErr};
use interp::Instr;
use log::Logger;
use mpu::Mpu;
use spu::Spu;
use unit::{Message, Unit};


pub struct Machine;

impl Machine {
    pub fn run(backend: Sender<Message>,
               bus_send: Sender<Message>,
               bus_recv: Receiver<Message>,
               instrs: &[Instr],
               logger: Logger)
               -> Result<bool, JezErr> {
        let mut funcs = HashMap::new();
        for (pc, instr) in instrs.iter().enumerate() {
            if let Instr::Begin(word) = *instr {
                // FIXME: Advance past `Begin` so that functions at instr zero
                //        are still callable.
                //
                // This is becuase the `call` instr is taking into account the
                // auto incrementing of `pc` by subtracting 1, which is
                // problematic if the callable exists at zero...
                funcs.insert(word, pc + 1);
            }
        }

        let (spu, spu_recv) = channel();
        let (mpu, mpu_recv) = channel();

        let spu_thread =
            match Spu::new("spu", instrs, &funcs, bus_send.clone(), spu_recv) {
                Some(mut unit) => {
                    Some(thread::spawn(move || {
                                           let res = Duration::new(0, 1000000);
                                           unit.run_forever(res);
                                       }))
                }
                None => None,
            };

        let mpu_thread =
            match Mpu::new("mpu", instrs, &funcs, bus_send.clone(), mpu_recv) {
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
                    println!("Unit '{}' has crashed {}", unit, err);
                    return Err(err);
                }
            }
        }

        Ok(false)
    }
}
