use std::sync::{Arc, Mutex, RwLock, RwLockWriteGuard, RwLockReadGuard};
use std::clone::Clone;

#[derive(Clone, Copy, Debug, PartialEq)]
struct RingState {
    writer: usize,
    reader: usize,
    started: bool,
}

// XXX: This will need to be lock free, cos audio. come back and fix (or replace
//      this) later, once needs have been fleshed out more...
#[derive(Clone, Debug)]
pub struct RingBuffer<T> {
    pos: Arc<Mutex<RwLock<RingState>>>,
    buff: Arc<Vec<RwLock<T>>>,
}

impl<T> RingBuffer<T>
    where T: Clone
{
    pub fn new(len: usize, value: T) -> RingBuffer<T> {
        let mut buff = Vec::with_capacity(len);
        for _ in 0..len {
            buff.push(RwLock::new(value.clone()));
        }

        let range = RingState {
            writer: 0,
            reader: 0,
            started: false,
        };

        RingBuffer {
            pos: Arc::new(Mutex::new(RwLock::new(range))),
            buff: Arc::new(buff),
        }
    }

    pub fn advance_write(&mut self) -> Option<RwLockWriteGuard<T>> {
        let lock = self.pos.lock().unwrap();
        let mut pos = lock.write().unwrap();

        if pos.started && pos.writer == pos.reader {
            return None;
        }

        let item = self.buff[pos.writer].write().unwrap();
        pos.writer = (pos.writer + 1) % self.buff.len();
        pos.started = true;
        Some(item)
    }

    pub fn advance_read(&self) -> Option<RwLockReadGuard<T>> {
        let lock = self.pos.lock().unwrap();
        let mut pos = lock.write().unwrap();

        if !pos.started && pos.writer == pos.reader {
            return None;
        }

        let item = self.buff[pos.reader].read().unwrap();
        pos.reader = (pos.reader + 1) % self.buff.len();
        if pos.writer == pos.reader {
            pos.started = false;
        }

        Some(item)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use vm::AudioBlock;
    use std::thread;

    #[test]
    fn test_threads() {
        let rb: RingBuffer<u64> = RingBuffer::new(3, 0);
        let mut producer = rb.clone();

        let res = thread::spawn(move || {
                                    assert!(producer.advance_write().is_some());
                                    assert!(producer.advance_write().is_some());
                                    assert!(producer.advance_write().is_some());
                                });

        res.join().unwrap();
        assert!(rb.advance_read().is_some());
        assert!(rb.advance_read().is_some());
        assert!(rb.advance_read().is_some());
        assert!(rb.advance_read().is_none());
    }

    #[test]
    fn test_xrun_write() {
        let mut rb: RingBuffer<u64> = RingBuffer::new(3, 0);
        assert!(rb.advance_write().is_some());
        assert!(rb.advance_write().is_some());
        assert!(rb.advance_write().is_some());
        assert!(rb.advance_write().is_none());
    }

    #[test]
    fn test_wrap_around() {
        let rb: RingBuffer<u64> = RingBuffer::new(3, 0);
        assert!(rb.advance_read().is_none());

        {
            let mut rb = rb.clone();
            assert!(rb.advance_write().is_some());
            assert!(rb.advance_write().is_some());
        }

        assert!(rb.advance_read().is_some());
        assert!(rb.advance_read().is_some());
        assert!(rb.advance_read().is_none());

        {
            let mut rb = rb.clone();
            assert!(rb.advance_write().is_some());
            assert!(rb.advance_write().is_some());
            assert!(rb.advance_write().is_some());
        }

        assert!(rb.advance_read().is_some());
        assert!(rb.advance_read().is_some());
        assert!(rb.advance_read().is_some());
        assert!(rb.advance_read().is_none());
    }

    #[test]
    fn test_multiple_consumers() {
        let block = AudioBlock::new(10);
        let rb: RingBuffer<AudioBlock> = RingBuffer::new(12, block);

        {
            let mut rb = rb.clone();
            let mut block = rb.advance_write().unwrap();
            let data = block.as_mut_slice();
            data[4] = 11.0;
        }

        {
            let mut rb = rb.clone();
            let mut block = rb.advance_write().unwrap();
            let data = block.as_mut_slice();
            data[2] = 10.0;
        }

        {
            let rb = rb.clone();
            let block = rb.advance_read().unwrap();
            let data = block.as_slice();
            assert_eq!(data[4], 11.0);

            let block = rb.advance_read().unwrap();
            let data = block.as_slice();
            assert_eq!(data[2], 10.0);
        }
    }
}
