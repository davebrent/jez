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


fn to_function_map(instrs: &[Instr]) -> HashMap<u64, usize> {
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
    funcs
}

pub struct Machine {
    backend: Sender<Message>,
    mpu: Sender<Message>,
    spu: Sender<Message>,
}

impl Machine {
    pub fn simulate(length: Duration,
                    dt: Duration,
                    backend: Sender<Message>,
                    bus_send: Sender<Message>,
                    bus_recv: Receiver<Message>,
                    instrs: &[Instr],
                    logger: Logger)
                    -> Result<bool, JezErr> {
        let funcs = to_function_map(instrs);
        let (spu, spu_recv) = channel();
        let (mpu, mpu_recv) = channel();
        let mut mach = Machine {
            backend: backend,
            spu: spu,
            mpu: mpu,
        };

        let mut _spu =
            Spu::new("spu", instrs, &funcs, bus_send.clone(), spu_recv);
        let mut _mpu =
            Mpu::new("mpu", instrs, &funcs, bus_send.clone(), mpu_recv);

        let mut elapsed = Duration::new(0, 0);
        while elapsed < length {
            while let Ok(msg) = bus_recv.try_recv() {
                logger.log(elapsed, "vm", &msg);
                let res = try!(mach.cycle(msg));
                match res {
                    None => continue,
                    Some(reload) => {
                        return Ok(reload);
                    }
                }
            }
            if let Some(mut unit) = _spu.as_mut() {
                unit.tick(&dt);
            }
            if let Some(mut unit) = _mpu.as_mut() {
                unit.tick(&dt);
            }
            elapsed += dt;
        }

        Ok(false)
    }

    pub fn realtime(backend: Sender<Message>,
                    bus_send: Sender<Message>,
                    bus_recv: Receiver<Message>,
                    instrs: &[Instr],
                    logger: Logger)
                    -> Result<bool, JezErr> {
        let funcs = to_function_map(instrs);
        let (spu, spu_recv) = channel();
        let (mpu, mpu_recv) = channel();
        let mut mach = Machine {
            backend: backend,
            spu: spu,
            mpu: mpu,
        };

        let _spu = Spu::new("spu", instrs, &funcs, bus_send.clone(), spu_recv);
        let _mpu = Mpu::new("mpu", instrs, &funcs, bus_send.clone(), mpu_recv);

        let spu_thread = match _spu {
            None => None,
            Some(mut unit) => {
                Some(thread::spawn(move || {
                                       let res = Duration::new(0, 1000000);
                                       unit.run_forever(res);
                                   }))
            }
        };

        let mpu_thread = match _mpu {
            None => None,
            Some(mut unit) => {
                Some(thread::spawn(move || {
                                       let res = Duration::new(0, 1000000);
                                       unit.run_forever(res);
                                   }))
            }
        };

        let start = Instant::now();
        while let Ok(msg) = bus_recv.recv() {
            logger.log(Instant::now() - start, "vm", &msg);
            let res = try!(mach.cycle(msg));
            match res {
                None => continue,
                Some(reload) => {
                    if let Some(thread) = spu_thread {
                        thread.join().unwrap();
                    }
                    if let Some(thread) = mpu_thread {
                        thread.join().unwrap();
                    }
                    return Ok(reload);
                }
            }
        }

        Ok(false)
    }

    fn cycle(&mut self, msg: Message) -> Result<Option<bool>, JezErr> {
        match msg {
            Message::Stop => {
                self.mpu.send(Message::Stop).unwrap();
                self.spu.send(Message::Stop).unwrap();
                Ok(Some(false))
            }
            Message::Reload => {
                self.mpu.send(Message::Stop).unwrap();
                self.spu.send(Message::Stop).unwrap();
                Ok(Some(true))
            }
            Message::MidiNoteOn(chan, pitch, vel) => {
                let msg = Message::MidiNoteOn(chan, pitch, vel);
                if self.backend.send(msg).is_err() {
                    Err(From::from(SysErr::UnreachableBackend))
                } else {
                    Ok(None)
                }
            }
            Message::MidiNoteOff(chan, pitch) => {
                let msg = Message::MidiNoteOff(chan, pitch);
                if self.backend.send(msg).is_err() {
                    Err(From::from(SysErr::UnreachableBackend))
                } else {
                    Ok(None)
                }
            }
            Message::MidiCtl(chan, ctl, val) => {
                let msg = Message::MidiCtl(chan, ctl, val);
                if self.backend.send(msg).is_err() {
                    Err(From::from(SysErr::UnreachableBackend))
                } else {
                    Ok(None)
                }
            }
            Message::SeqEvent(event) => {
                match self.mpu.send(Message::SeqEvent(event)) {
                    _ => Ok(None)
                }
            }
            Message::Error(_, err) => Err(err),
        }
    }
}
