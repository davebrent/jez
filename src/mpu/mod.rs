mod words;
mod state;

use std::convert::From;
use std::collections::HashMap;
use std::sync::mpsc::{Receiver, Sender};
use std::time::Duration;

use assem::hash_str;
use err::RuntimeErr;
use interp::{Instr, Interpreter, InterpResult, InterpState};
use math::{Curve, dur_to_millis, millis_to_dur, point_on_curve};
use unit::{Event, EventValue, Message, Unit};

use self::state::{MidiMessage, MidiState};
use self::words::{ctrlout, event_duration, event_track, event_value, noteout};


type MpuKeyword = fn(&mut MidiState, &mut InterpState) -> InterpResult;

pub struct Mpu {
    id: &'static str,
    interp: Interpreter<MidiState>,
    channel: Sender<Message>,
    input_channel: Receiver<Message>,
    instrs_out_note: Option<usize>,
    instrs_out_ctrl: Option<usize>,
    off_events: Vec<(Duration, u8, u8)>,
    ctl_events: Vec<(Duration, f64, u8, u8, Curve)>,
}

impl Mpu {
    pub fn new(id: &'static str,
               instrs: &[Instr],
               funcs: &HashMap<u64, usize>,
               channel: Sender<Message>,
               input_channel: Receiver<Message>)
               -> Option<Self> {
        let out_note = funcs.get(&hash_str("mpu_out_note")).cloned();
        let out_ctrl = funcs.get(&hash_str("mpu_out_ctrl")).cloned();

        if out_note.is_none() && out_ctrl.is_none() {
            return None;
        }

        let mut words: HashMap<&'static str, MpuKeyword> = HashMap::new();
        words.insert("ctrlout", ctrlout);
        words.insert("event_duration", event_duration);
        words.insert("event_track", event_track);
        words.insert("event_value", event_value);
        words.insert("noteout", noteout);

        Some(Mpu {
                 id: id,
                 interp: Interpreter::new(instrs.to_vec(),
                                          words,
                                          MidiState::new()),
                 channel: channel,
                 input_channel: input_channel,
                 instrs_out_note: out_note,
                 instrs_out_ctrl: out_ctrl,
                 off_events: Vec::new(),
                 ctl_events: Vec::new(),
             })
    }

    fn handle_trg_event(&mut self, event: Event) {
        if self.instrs_out_note.is_none() {
            return;
        }

        let instrs = self.instrs_out_note.unwrap();
        self.interp.state.reset();
        self.interp.data.event = event;
        self.interp.data.message = MidiMessage::None;

        match self.interp.eval(instrs) {
            Err(err) => {
                self.channel
                    .send(Message::Error(self.id, From::from(err)))
                    .unwrap();
            }
            Ok(_) => {
                match self.interp.data.message {
                    MidiMessage::None => return,
                    MidiMessage::Note {
                        channel: chan,
                        pitch: ptch,
                        velocity: vel,
                        duration: dur,
                    } => {
                        let len = self.off_events.len();
                        self.off_events
                            .retain(|&evt| !(evt.1 == chan && evt.2 == ptch));
                        if len != self.off_events.len() {
                            self.channel
                                .send(Message::MidiNoteOff(chan, ptch))
                                .unwrap();
                        }

                        self.off_events.push((millis_to_dur(dur), chan, ptch));
                        self.off_events
                            .sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap());
                        self.channel
                            .send(Message::MidiNoteOn(chan, ptch, vel))
                            .unwrap();
                    }
                    _ => {
                        let err = RuntimeErr::InvalidArgs;
                        self.channel
                            .send(Message::Error(self.id, From::from(err)))
                            .unwrap();
                    }
                }
            }
        }
    }

    fn handle_ctl_event(&mut self, event: Event, curve: Curve) {
        if self.instrs_out_ctrl.is_none() {
            return;
        }

        let instrs = self.instrs_out_ctrl.unwrap();
        self.interp.state.reset();
        self.interp.data.event = event;
        self.interp.data.message = MidiMessage::None;

        match self.interp.eval(instrs) {
            Err(err) => {
                self.channel
                    .send(Message::Error(self.id, From::from(err)))
                    .unwrap();
            }
            Ok(_) => {
                match self.interp.data.message {
                    MidiMessage::None => return,
                    MidiMessage::Ctrl {
                        channel: chan,
                        ctrl: ctl,
                    } => {
                        let dur = millis_to_dur(event.dur);
                        let msg = Message::MidiCtl(chan, ctl, curve[0] as u8);
                        self.ctl_events.push((dur, 0.0, chan, ctl, curve));
                        self.ctl_events
                            .sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap());
                        self.channel.send(msg).unwrap();
                    }
                    _ => {
                        let err = RuntimeErr::InvalidArgs;
                        self.channel
                            .send(Message::Error(self.id, From::from(err)))
                            .unwrap();
                    }
                }
            }
        }
    }

    fn process_ctl_events(&mut self, delta: &Duration) {
        for evt in &mut self.ctl_events {
            evt.1 += dur_to_millis(delta) / dur_to_millis(&evt.0);
        }

        for evt in &mut self.ctl_events {
            let t = evt.1;
            let val = point_on_curve(t, &evt.4);
            let msg = Message::MidiCtl(evt.2, evt.3, val[1] as u8);
            self.channel.send(msg).unwrap();
        }

        self.ctl_events.retain(|&evt| evt.1 < 1.0);
    }

    fn process_off_events(&mut self, delta: &Duration) {
        let zero = Duration::new(0, 0);

        for evt in &mut self.off_events {
            evt.0 = match evt.0.checked_sub(*delta) {
                Some(dur) => dur,
                None => zero,
            }
        }

        while let Some((dur, chan, pitch)) = self.off_events.pop() {
            if dur != zero {
                self.off_events.push((dur, chan, pitch));
                break;
            } else {
                let msg = Message::MidiNoteOff(chan, pitch);
                self.channel.send(msg).unwrap();
            }
        }
    }

    fn dispatch_off_events(&mut self) {
        while let Some((_, chan, pitch)) = self.off_events.pop() {
            self.channel
                .send(Message::MidiNoteOff(chan, pitch))
                .unwrap();
        }
    }
}

impl Unit for Mpu {
    fn tick(&mut self, delta: &Duration) -> bool {
        self.process_ctl_events(delta);
        self.process_off_events(delta);

        if let Ok(msg) = self.input_channel.try_recv() {
            match msg {
                Message::Stop => {
                    self.dispatch_off_events();
                    return true;
                }
                Message::SeqEvent(event) => {
                    match event.value {
                        EventValue::Trigger(_) => self.handle_trg_event(event),
                        EventValue::Curve(curve) => {
                            self.handle_ctl_event(event, curve)
                        }
                    }
                }
                _ => (),
            }
        }

        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::sync::mpsc::channel;

    use math::millis_to_dur;
    use unit::{Event, EventValue, Message};

    fn get_test_instrs() -> (Vec<Instr>, HashMap<u64, usize>) {
        let mut map = HashMap::new();
        map.insert(hash_str("mpu_out_note"), 1);
        (vec![Instr::Begin(hash_str("mpu_out_note")),
              Instr::Keyword(hash_str("event_value")),
              Instr::LoadNumber(127.0),
              Instr::Keyword(hash_str("event_duration")),
              Instr::LoadNumber(1.0),
              Instr::Keyword(hash_str("noteout")),
              Instr::End(hash_str("mpu_out_note"))],
         map)
    }

    #[test]
    fn test_simple_note_off_events() {
        // Tests the note events for two scheduled events
        let (instrs, map) = get_test_instrs();
        let (in_tx, in_rx) = channel();
        let (out_tx, out_rx) = channel();
        let mut mpu = Mpu::new("mpu", &instrs, &map, out_tx, in_rx).unwrap();

        in_tx
            .send(Message::SeqEvent(Event {
                                        track: 1,
                                        onset: 0.0,
                                        dur: 100.0,
                                        value: EventValue::Trigger(64.0),
                                    }))
            .unwrap();

        mpu.tick(&Duration::new(0, 0));
        assert_eq!(out_rx.recv().unwrap(), Message::MidiNoteOn(1, 64, 127));

        mpu.tick(&millis_to_dur(99.0));
        assert_eq!(out_rx.try_recv().is_err(), true);

        mpu.tick(&millis_to_dur(1.0));
        assert_eq!(out_rx.try_recv().unwrap(), Message::MidiNoteOff(1, 64));

        in_tx
            .send(Message::SeqEvent(Event {
                                        track: 1,
                                        onset: 100.0,
                                        dur: 200.0,
                                        value: EventValue::Trigger(96.0),
                                    }))
            .unwrap();
        mpu.tick(&Duration::new(0, 0));
        assert_eq!(out_rx.recv().unwrap(), Message::MidiNoteOn(1, 96, 127));

        mpu.tick(&millis_to_dur(199.0));
        assert_eq!(out_rx.try_recv().is_err(), true);

        mpu.tick(&millis_to_dur(1.0));
        assert_eq!(out_rx.try_recv().unwrap(), Message::MidiNoteOff(1, 96));
    }

    #[test]
    fn test_flush_single_note_off() {
        // Tests that a note off event is sent, if the same note has been
        // newly triggered, so that the new event is not cut short
        let (instrs, map) = get_test_instrs();
        let (in_tx, in_rx) = channel();
        let (out_tx, out_rx) = channel();
        let mut mpu = Mpu::new("mpu", &instrs, &map, out_tx, in_rx).unwrap();

        in_tx
            .send(Message::SeqEvent(Event {
                                        track: 1,
                                        onset: 0.0,
                                        dur: 100.0,
                                        value: EventValue::Trigger(64.0),
                                    }))
            .unwrap();

        mpu.tick(&Duration::new(0, 0));
        let evt = out_rx.recv().unwrap();
        assert_eq!(evt, Message::MidiNoteOn(1, 64, 127));
        mpu.tick(&millis_to_dur(50.0));

        in_tx
            .send(Message::SeqEvent(Event {
                                        track: 1,
                                        onset: 0.0,
                                        dur: 100.0,
                                        value: EventValue::Trigger(64.0),
                                    }))
            .unwrap();

        mpu.tick(&Duration::new(0, 0));
        assert_eq!(out_rx.recv().unwrap(), Message::MidiNoteOff(1, 64));
        assert_eq!(out_rx.recv().unwrap(), Message::MidiNoteOn(1, 64, 127));

        mpu.tick(&millis_to_dur(99.0));
        assert_eq!(out_rx.try_recv().is_err(), true);
        mpu.tick(&millis_to_dur(1.0));
        assert_eq!(out_rx.recv().unwrap(), Message::MidiNoteOff(1, 64));
    }

    #[test]
    fn test_stopping() {
        let instrs = vec![Instr::Begin(hash_str("mpu_out_note")),
                          Instr::End(hash_str("mpu_out_note"))];
        let mut map = HashMap::new();
        map.insert(hash_str("mpu_out_note"), 0);

        let (in_tx, in_rx) = channel();
        let (out_tx, _) = channel();
        let mut mpu = Mpu::new("mpu", &instrs, &map, out_tx, in_rx).unwrap();
        in_tx.send(Message::Stop).unwrap();
        mpu.run_forever(Duration::new(0, 1000000));
        assert_eq!(true, true);
    }
}
