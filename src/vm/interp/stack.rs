use std::collections::HashMap;

use serde::Serialize;

use super::types::Value;
use crate::err::Error;

#[derive(Clone, Debug, Serialize)]
pub struct StackFrame {
    pub stack: Vec<Value>,
    pub locals: HashMap<u64, usize>,
    pub begin: usize,
    pub ret_addr: usize,
}

impl StackFrame {
    pub fn new(begin: usize, ret_addr: usize) -> StackFrame {
        StackFrame {
            stack: Vec::new(),
            begin: begin,
            ret_addr: ret_addr,
            locals: HashMap::new(),
        }
    }

    pub fn last(&self) -> Result<Value, Error> {
        match self.stack.last() {
            Some(val) => Ok(val.clone()),
            None => Err(error!(StackExhausted)),
        }
    }

    pub fn pop(&mut self) -> Result<Value, Error> {
        match self.stack.pop() {
            Some(val) => Ok(val),
            None => Err(error!(StackExhausted)),
        }
    }

    pub fn push(&mut self, val: Value) -> Result<(), Error> {
        self.stack.push(val);
        Ok(())
    }
}
