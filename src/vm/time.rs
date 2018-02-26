use std::clone::Clone;
use std::fmt::Debug;
use std::sync::mpsc::{Receiver, Sender};
use std::thread;
use std::time::{Duration, Instant};

use super::math::millis_to_dur;

#[derive(Clone, Copy, Debug, PartialEq)]
#[allow(dead_code)]
pub enum TimeEvent<T>
where
    T: Clone + Debug,
{
    Stop,
    Timer(Duration, T),
    Timeout(f64, T),
    Interval(f64, T),
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct Timer<T>
where
    T: Clone + Debug,
{
    pub dur: Duration,
    pub elapsed: Duration,
    pub dispatched_at: Duration,
    pub recurring: bool,
    pub data: T,
}

#[derive(Debug)]
pub struct Clock<T>
where
    T: Clone + Debug,
{
    input: Receiver<TimeEvent<T>>,
    output: Sender<TimeEvent<T>>,
    timers: Vec<Timer<T>>,
    enabled: bool,
    elapsed: Duration,
}

impl<T> Clock<T>
where
    T: Clone + Debug,
{
    pub fn new(output: Sender<TimeEvent<T>>, input: Receiver<TimeEvent<T>>) -> Clock<T> {
        Clock {
            input: input,
            output: output,
            timers: Vec::new(),
            enabled: false,
            elapsed: Duration::new(0, 0),
        }
    }

    pub fn update(&mut self, delta: &Duration) {
        self.elapsed += *delta;

        for timer in &mut self.timers {
            timer.elapsed += *delta;
        }

        while let Some(timer) = self.timers.pop() {
            if timer.elapsed < timer.dur {
                self.timers.push(timer);
                break;
            } else {
                let elapsed = self.elapsed;
                let data = timer.data.clone();
                self.output.send(TimeEvent::Timer(elapsed, data)).ok();

                if timer.recurring {
                    let mut next = timer;
                    next.elapsed = Duration::new(0, 0);
                    next.dispatched_at = self.elapsed + next.dur;
                    self.push_timer(next);
                }
            }
        }
    }

    pub fn timeout(&mut self, dur: f64, data: T) {
        let dur = millis_to_dur(dur);
        let elapsed = self.elapsed;
        self.push_timer(Timer {
            dur: dur,
            dispatched_at: elapsed + dur,
            elapsed: Duration::new(0, 0),
            data: data,
            recurring: false,
        });
    }

    pub fn interval(&mut self, dur: f64, data: T) {
        let dur = millis_to_dur(dur);
        let elapsed = self.elapsed;
        self.push_timer(Timer {
            dur: dur,
            dispatched_at: elapsed + dur,
            elapsed: Duration::new(0, 0),
            data: data,
            recurring: true,
        });
    }

    fn push_timer(&mut self, timer: Timer<T>) {
        self.timers.push(timer);
        self.timers
            .sort_by(|a, b| b.dispatched_at.partial_cmp(&a.dispatched_at).unwrap());
    }

    pub fn tick(&mut self, delta: &Duration) -> bool {
        self.update(delta);

        while let Ok(msg) = self.input.try_recv() {
            match msg {
                TimeEvent::Stop => return false,
                TimeEvent::Timeout(t, data) => self.timeout(t, data),
                TimeEvent::Interval(t, data) => self.interval(t, data),
                _ => continue,
            }
        }

        true
    }

    pub fn run_forever(&mut self) {
        let mut previous = Instant::now();
        let res = Duration::new(0, 1);
        loop {
            let now = Instant::now();
            let delta = now.duration_since(previous);

            if self.tick(&delta) {
                previous = now;
                // Sleeping instead of `yield_now` to keep CPU usage down
                thread::sleep(res);
            } else {
                break;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::mpsc::channel;

    #[test]
    fn test_out_of_order_timeouts() {
        let (send1, recv1) = channel();
        let (_, recv2) = channel();

        let mut unit = Clock::new(send1, recv2);
        unit.timeout(0.0, 10);
        unit.timeout(100.0, 30);
        unit.timeout(10.0, 20);

        unit.tick(&millis_to_dur(5.0));
        let res = recv1.try_recv();
        assert!(res.is_ok());
        assert_eq!(res.unwrap(), TimeEvent::Timer(millis_to_dur(5.0), 10));
        assert!(recv1.try_recv().is_err());

        unit.tick(&millis_to_dur(5.0));
        let res = recv1.try_recv();
        assert!(res.is_ok());
        assert_eq!(res.unwrap(), TimeEvent::Timer(millis_to_dur(10.0), 20));
        assert!(recv1.try_recv().is_err());

        unit.tick(&millis_to_dur(90.0));
        let res = recv1.try_recv();
        assert!(res.is_ok());
        assert_eq!(res.unwrap(), TimeEvent::Timer(millis_to_dur(100.0), 30));
        assert!(recv1.try_recv().is_err());
    }

    #[test]
    fn test_intervals() {
        let (send1, recv1) = channel();
        let (_, recv2) = channel();

        let mut unit = Clock::new(send1, recv2);
        unit.interval(10.0, 10);
        unit.interval(20.0, 30);

        unit.tick(&millis_to_dur(5.0));
        assert!(recv1.try_recv().is_err());

        unit.tick(&millis_to_dur(5.0));
        let res = recv1.try_recv();
        assert!(res.is_ok());
        assert_eq!(res.unwrap(), TimeEvent::Timer(millis_to_dur(10.0), 10));
        assert!(recv1.try_recv().is_err());

        unit.tick(&millis_to_dur(5.0));
        assert!(recv1.try_recv().is_err());

        unit.tick(&millis_to_dur(5.0));
        let res = recv1.try_recv();
        assert!(res.is_ok());
        assert_eq!(res.unwrap(), TimeEvent::Timer(millis_to_dur(20.0), 10));
        let res = recv1.try_recv();
        assert!(res.is_ok());
        assert_eq!(res.unwrap(), TimeEvent::Timer(millis_to_dur(20.0), 30));

        assert!(recv1.try_recv().is_err());
    }
}
