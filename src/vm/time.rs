use std::clone::Clone;
use std::cmp::{Eq, Ord, Ordering, PartialOrd};
use std::collections::BinaryHeap;
use std::fmt::Debug;
use std::sync::mpsc::{Receiver, Sender};
use std::thread;
use std::time::{Duration, Instant};

pub trait Priority {
    fn priority(&self) -> usize;
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Schedule<T>
where
    T: Copy + Clone + Debug + Priority,
{
    Stop,
    At(f64, T),
}

#[derive(Clone, Copy, Debug)]
struct Timer<T>
where
    T: Copy + Clone + Debug + Priority,
{
    pub t: Duration,
    pub interval: Option<Duration>,
    pub data: T,
}

impl<T> PartialEq for Timer<T>
where
    T: Copy + Clone + Debug + Priority,
{
    fn eq(&self, other: &Timer<T>) -> bool {
        self.t == other.t
    }
}

impl<T> Eq for Timer<T> where T: Copy + Clone + Debug + Priority {}

impl<T> PartialOrd for Timer<T>
where
    T: Copy + Clone + Debug + Priority,
{
    fn partial_cmp(&self, other: &Timer<T>) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl<T> Ord for Timer<T>
where
    T: Copy + Clone + Debug + Priority,
{
    fn cmp(&self, other: &Timer<T>) -> Ordering {
        let order = self.t.cmp(&other.t).reverse();
        match order {
            Ordering::Equal => {
                let a = self.data.priority();
                let b = other.data.priority();
                a.cmp(&b).reverse()
            }
            _ => order,
        }
    }
}

pub fn millis_to_dur(millis: f64) -> Duration {
    let secs = (millis / 1000.0).floor();
    let nanos = (millis - (secs * 1000.0)) * 1000000.0;
    Duration::new(secs as u64, nanos as u32)
}

pub fn dur_to_millis(dur: Duration) -> f64 {
    let secs = dur.as_secs() as f64 * 1000.0;
    let nanos = f64::from(dur.subsec_nanos()) / 1000000.0;
    secs + nanos
}

#[derive(Debug)]
pub struct Clock<T>
where
    T: Copy + Clone + Debug + Priority,
{
    input: Receiver<Schedule<T>>,
    output: Sender<Schedule<T>>,
    timers: BinaryHeap<Timer<T>>,
    elapsed: Duration,
}

impl<T> Clock<T>
where
    T: Copy + Clone + Debug + Priority,
{
    pub fn new(output: Sender<Schedule<T>>, input: Receiver<Schedule<T>>) -> Clock<T> {
        Clock {
            input: input,
            output: output,
            timers: BinaryHeap::new(),
            elapsed: Duration::new(0, 0),
        }
    }

    pub fn timeout(&mut self, t: f64, data: T) {
        let t = millis_to_dur(t);
        self.timers.push(Timer {
            t: t,
            data: data,
            interval: None,
        });
    }

    pub fn interval(&mut self, t: f64, data: T) {
        let t = millis_to_dur(t);
        self.timers.push(Timer {
            t: t,
            data: data,
            interval: Some(t),
        });
    }

    fn next(&mut self) -> Option<Timer<T>> {
        if match self.timers.peek() {
            Some(timer) => timer.t <= self.elapsed,
            None => false,
        } {
            self.timers.pop()
        } else {
            None
        }
    }

    pub fn tick(&mut self, delta: Duration) -> bool {
        // Read input
        while let Ok(msg) = self.input.try_recv() {
            match msg {
                Schedule::At(t, data) => self.timeout(t, data),
                Schedule::Stop => return false,
            };
        }

        // Update elapsed time
        self.elapsed += delta;
        let elapsed = dur_to_millis(self.elapsed);

        // Process timers
        while let Some(timer) = self.next() {
            let expected = dur_to_millis(timer.t);
            let event = Schedule::At(elapsed, timer.data);
            self.output.send(event).ok();

            let error = (elapsed - expected).abs();
            if error > 1.0 {
                eprintln!("Event dispatched at incorrect time, off by {}ms", error);
            }

            if let Some(interval) = timer.interval {
                let mut next = timer;
                next.t = timer.t + interval;
                self.timers.push(next);
            }
        }

        true
    }

    pub fn run_forever(&mut self) {
        let mut previous = Instant::now();
        let priority_time = millis_to_dur(1.5);
        let default_sleep = millis_to_dur(20.0);

        loop {
            let now = Instant::now();
            let delta = now.duration_since(previous);
            previous = now;

            if !self.tick(delta) {
                break;
            }

            let target_time = match self.timers.peek() {
                Some(timer) => match timer.t.checked_sub(self.elapsed) {
                    Some(time) => time,
                    None => default_sleep,
                },
                None => default_sleep,
            };

            if target_time > priority_time {
                thread::sleep(target_time / 2);
            } else {
                for _ in 0..10 {
                    thread::yield_now();
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::mpsc::channel;

    #[derive(Clone, Copy, Debug, PartialEq)]
    struct Event(usize);

    impl Priority for Event {
        fn priority(&self) -> usize {
            self.0
        }
    }

    #[test]
    fn test_out_of_order_timeouts() {
        let (send1, recv1) = channel();
        let (_, recv2) = channel();

        let mut unit = Clock::new(send1, recv2);
        unit.timeout(100.0, Event(30));
        unit.timeout(10.0, Event(20));

        unit.tick(millis_to_dur(10.0));
        let res = recv1.try_recv();
        assert!(res.is_ok());
        assert_eq!(res.unwrap(), Schedule::At(10.0, Event(20)));
        assert!(recv1.try_recv().is_err());

        unit.tick(millis_to_dur(90.0));
        let res = recv1.try_recv();
        assert!(res.is_ok());
        assert_eq!(res.unwrap(), Schedule::At(100.0, Event(30)));
        assert!(recv1.try_recv().is_err());
    }

    #[test]
    fn test_intervals() {
        let (send1, recv1) = channel();
        let (_, recv2) = channel();

        let mut unit = Clock::new(send1, recv2);
        unit.interval(10.0, Event(10));
        unit.interval(20.0, Event(30));

        unit.tick(millis_to_dur(5.0));
        assert!(recv1.try_recv().is_err());

        unit.tick(millis_to_dur(5.0));
        let res = recv1.try_recv();
        assert!(res.is_ok());
        assert_eq!(res.unwrap(), Schedule::At(10.0, Event(10)));
        assert!(recv1.try_recv().is_err());

        unit.tick(millis_to_dur(5.0));
        assert!(recv1.try_recv().is_err());

        unit.tick(millis_to_dur(5.0));
        let res = recv1.try_recv();
        assert!(res.is_ok());
        assert_eq!(res.unwrap(), Schedule::At(20.0, Event(10)));
        let res = recv1.try_recv();
        assert!(res.is_ok());
        assert_eq!(res.unwrap(), Schedule::At(20.0, Event(30)));

        assert!(recv1.try_recv().is_err());
    }

    #[test]
    fn test_time_fns() {
        let dur = millis_to_dur(2500.0);
        assert_eq!(dur, Duration::new(2, 500000000));
        assert_eq!(dur_to_millis(dur), 2500.0);
    }
}
