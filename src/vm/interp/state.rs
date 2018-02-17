use std::collections::HashMap;

use err::RuntimeErr;

use super::types::{InterpResult, Value};
use super::stack::StackFrame;

#[derive(Clone, Debug, Serialize)]
pub struct InterpState {
    pub reserved: usize,
    pub heap: Vec<Value>,
    pub pc: usize,
    pub globals: HashMap<u64, usize>,
    pub strings: HashMap<u64, String>,
    pub frames: Vec<StackFrame>,
    pub exit: bool,
}

impl InterpState {
    pub fn new() -> InterpState {
        InterpState {
            pc: 0,
            reserved: 0,
            heap: vec![],
            globals: HashMap::new(),
            strings: HashMap::new(),
            frames: vec![],
            exit: false,
        }
    }

    fn frame(&self) -> Result<&StackFrame, RuntimeErr> {
        match self.frames.last() {
            None => Err(RuntimeErr::StackExhausted(None)),
            Some(frame) => Ok(frame),
        }
    }

    fn frame_mut(&mut self) -> Result<&mut StackFrame, RuntimeErr> {
        match self.frames.last_mut() {
            None => Err(RuntimeErr::StackExhausted(None)),
            Some(frame) => Ok(frame),
        }
    }

    pub fn heap_slice_mut(&mut self, start: usize, end: usize) -> Result<&mut [Value], RuntimeErr> {
        if start > end || end > self.heap_len() {
            return Err(RuntimeErr::InvalidArgs(None));
        }
        Ok(&mut self.heap[start..end])
    }

    pub fn heap_get(&self, ptr: usize) -> Result<Value, RuntimeErr> {
        match self.heap.get(ptr) {
            Some(val) => Ok(val.clone()),
            None => Err(RuntimeErr::InvalidArgs(None)),
        }
    }

    pub fn heap_len(&self) -> usize {
        self.heap.len()
    }

    pub fn heap_push(&mut self, val: Value) -> usize {
        self.heap.push(val);
        self.heap_len()
    }

    pub fn call(&mut self, loc: usize, args: usize, pc: usize) -> InterpResult {
        // Push a new stack frame copying across any arguments, if any, from
        // the previous frame
        let mut frame = StackFrame::new(loc, self.pc);
        if args != 0 {
            let caller = try!(self.frame_mut());
            for _ in 0..args {
                try!(frame.push(try!(caller.pop())));
            }
        }
        self.frames.push(frame);
        // Account for implicit increment of pc
        self.pc = pc - 1;
        Ok(None)
    }

    pub fn ret(&mut self) -> InterpResult {
        match self.frames.pop() {
            None => Err(RuntimeErr::StackExhausted(None)),
            Some(mut frame) => {
                // If this is the last stack frame, return the 'top of stack'
                // value as the final result. Otherwise 'None' and continue
                // evaluating instructions
                let res = match frame.pop() {
                    Ok(val) => val,
                    Err(_) => Value::Null,
                };

                if self.frames.is_empty() {
                    self.exit = true;
                    Ok(Some(res))
                } else {
                    try!(self.push(res));
                    self.pc = frame.ret_addr;
                    Ok(None)
                }
            }
        }
    }

    pub fn last(&self) -> Result<Value, RuntimeErr> {
        let frame = try!(self.frame());
        Ok(try!(frame.last()))
    }

    pub fn pop(&mut self) -> Result<Value, RuntimeErr> {
        let frame = try!(self.frame_mut());
        Ok(try!(frame.pop()))
    }

    pub fn pop_num(&mut self) -> Result<f64, RuntimeErr> {
        match try!(self.pop()) {
            Value::Number(num) => Ok(num),
            _ => Err(RuntimeErr::InvalidArgs(None)),
        }
    }

    pub fn push(&mut self, val: Value) -> InterpResult {
        match self.frames.last_mut() {
            None => Err(RuntimeErr::StackExhausted(None)),
            Some(frame) => {
                try!(frame.push(val));
                Ok(None)
            }
        }
    }

    pub fn store(&mut self, name: u64, val: Value) -> Result<(), RuntimeErr> {
        let ptr = self.heap_len();
        self.heap_push(val);
        match self.frames.last_mut() {
            Some(frame) => frame.locals.insert(name, ptr),
            None => self.globals.insert(name, ptr),
        };
        Ok(())
    }

    pub fn store_glob(&mut self, name: u64, val: Value) -> Result<(), RuntimeErr> {
        let ptr = self.heap_len();
        self.heap_push(val);
        self.globals.insert(name, ptr);
        Ok(())
    }

    pub fn lookup(&mut self, name: u64) -> Result<Value, RuntimeErr> {
        let ptr = match self.frame() {
            Ok(frame) => frame.locals.get(&name),
            Err(_) => None,
        };
        let ptr = match ptr {
            Some(ptr) => Some(ptr),
            None => self.globals.get(&name),
        };
        match ptr {
            Some(ptr) => self.heap_get(*ptr),
            None => Err(RuntimeErr::InvalidArgs(None)),
        }
    }

    pub fn reset(&mut self) {
        self.exit = false;
        self.heap.truncate(self.reserved);;
    }
}
