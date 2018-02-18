use std::collections::HashMap;
use std::fmt::Write;

use err::Error;
use lang::hash_str;

pub use super::types::{Instr, InterpResult, Value};
pub use super::stack::StackFrame;
pub use super::state::InterpState;

pub type Keyword<S> = fn(&mut S, &mut InterpState) -> InterpResult;

pub trait Interpreter<S> {
    /// Reset the interpreters internal state
    fn reset(&mut self);

    /// Return a mutable reference to user/extension data
    fn data_mut(&mut self) -> &mut S;

    /// Return a copy of the interpreters internal state
    fn state(&self) -> InterpState;

    /// Return all the interpreters instructions
    fn instrs(&self) -> &[Instr];

    /// Execute a single instruction
    fn execute(&mut self, pc: usize, instr: Instr) -> InterpResult;

    /// Evaluate all instructions from a program counter
    fn eval(&mut self, pc: usize) -> InterpResult;

    /// Evaluate a block of instructions
    fn eval_block(&mut self, block: u64) -> InterpResult {
        let instrs = self.instrs().to_vec();
        for (pc, instr) in instrs.iter().enumerate() {
            if let Instr::Begin(word) = *instr {
                if word == block {
                    return self.eval(pc + 1);
                }
            }
        }
        Err(exception!())
    }
}

fn list_begin(state: &mut InterpState, begin: Instr) -> InterpResult {
    let val = Value::Instruction(begin);
    try!(state.push(val));
    Ok(None)
}

fn list_end<F>(state: &mut InterpState, end: Instr, con: F) -> InterpResult
where
    F: Fn(usize, usize) -> Value,
{
    let start = state.heap_len();
    loop {
        let val = try!(state.pop());
        match val {
            Value::Instruction(instr) => {
                if instr == end {
                    let end = state.heap_len();
                    try!(state.push(con(start, end)));
                    try!(state.heap_slice_mut(start, end)).reverse();
                    return Ok(None);
                } else {
                    state.heap_push(val)
                }
            }
            _ => state.heap_push(val),
        };
    }
}

pub struct BaseInterpreter<S> {
    data: S,
    state: InterpState,
    instrs: Vec<Instr>,
    words: HashMap<u64, Keyword<S>>,
}

impl<S> BaseInterpreter<S> {
    pub fn new(
        instrs: Vec<Instr>,
        exts: &HashMap<&'static str, Keyword<S>>,
        data: S,
    ) -> BaseInterpreter<S> {
        let mut words = HashMap::new();
        for (word, func) in exts {
            words.insert(hash_str(word), *func);
        }

        let mut interpreter = BaseInterpreter {
            instrs: instrs,
            words: words,
            data: data,
            state: InterpState::new(),
        };

        interpreter.eval_block(0).ok();
        interpreter.state.reserved = interpreter.state.heap.len();
        interpreter.state.reset();
        interpreter
    }
}

impl<S> Interpreter<S> for BaseInterpreter<S> {
    fn instrs(&self) -> &[Instr] {
        &self.instrs
    }

    fn state(&self) -> InterpState {
        self.state.clone()
    }

    fn data_mut(&mut self) -> &mut S {
        &mut self.data
    }

    fn reset(&mut self) {
        self.state.reset();
    }

    fn execute(&mut self, pc: usize, instr: Instr) -> InterpResult {
        match instr {
            Instr::Null => self.state.push(Value::Null),

            Instr::LoadNumber(n) => self.state.push(Value::Number(n)),

            Instr::LoadSymbol(s) => self.state.push(Value::Symbol(s)),

            Instr::Call(args, ret) => self.state.call(pc, args, ret),

            Instr::Return => self.state.ret(),

            Instr::StoreGlob(name) => {
                let val = try!(self.state.pop());
                try!(self.state.store_glob(name, val));
                Ok(None)
            }

            Instr::StoreVar(name) => {
                let val = try!(self.state.pop());
                try!(self.state.store(name, val));
                Ok(None)
            }

            Instr::LoadVar(name) => {
                let val = try!(self.state.lookup(name));
                try!(self.state.push(val));
                Ok(None)
            }

            Instr::ListBegin => list_begin(&mut self.state, Instr::ListBegin),
            Instr::ListEnd => list_end(&mut self.state, Instr::ListBegin, |start, end| {
                Value::List(start, end)
            }),

            Instr::SeqBegin => list_begin(&mut self.state, Instr::SeqBegin),
            Instr::SeqEnd => list_end(&mut self.state, Instr::SeqBegin, |start, end| {
                Value::Seq(start, end)
            }),

            Instr::GroupBegin => list_begin(&mut self.state, Instr::GroupBegin),
            Instr::GroupEnd => list_end(&mut self.state, Instr::GroupBegin, |start, end| {
                Value::Group(start, end)
            }),

            Instr::Keyword(word) => {
                // Keywords operate on an implicit stack frame
                if let Some(func) = self.words.get(&word) {
                    func(&mut self.data, &mut self.state)
                } else {
                    Err(error!(UnknownKeyword))
                }
            }

            Instr::LoadString(id) => {
                // Look up the string and push to the stack
                let string = self.state.strings.get(&id).map(|s| s.clone());
                match string {
                    Some(string) => {
                        try!(self.state.push(Value::Str(string)));
                        Ok(None)
                    }
                    None => Err(error!(InvalidArgs)),
                }
            }

            Instr::StoreString(id, len) => {
                // Pull all bytes out of the succeeding raw data instructions
                // and interpret as a valid unicode string
                let mut bytes = Vec::with_capacity(len as usize);
                for i in 0..len {
                    let pc = self.state.pc + i as usize + 1;
                    match self.instrs[pc] {
                        Instr::RawData(byte) => bytes.push(byte),
                        _ => return Err(exception!()),
                    };
                }
                match String::from_utf8(bytes) {
                    Ok(string) => self.state.strings.insert(id, string),
                    Err(_) => return Err(exception!()),
                };
                self.state.pc += len as usize;
                Ok(None)
            }

            _ => Ok(None),
        }
    }

    fn eval(&mut self, pc: usize) -> InterpResult {
        try!(self.state.call(pc, 0, pc));
        while self.state.pc < self.instrs.len() && !self.state.exit {
            let pc = self.state.pc;
            let instr = self.instrs[pc];
            match try!(self.execute(pc, instr)) {
                None => (),
                Some(val) => return Ok(Some(val)),
            }
            self.state.pc += 1;
        }
        Ok(None)
    }
}

/// An interpreter that adds stack trace support to another interpreter
pub struct StackTraceInterpreter<S> {
    inner: Box<Interpreter<S>>,
}

impl<S> StackTraceInterpreter<S> {
    pub fn new(interp: Box<Interpreter<S>>) -> StackTraceInterpreter<S> {
        StackTraceInterpreter { inner: interp }
    }

    fn stack_trace(&self) -> String {
        let state = self.inner.state();
        // There should always be source loc strings created by the assembler
        assert!(!state.strings.is_empty());

        let mut msg = String::new();
        write!(&mut msg, "Traceback (most recent call last)").unwrap();
        for frame in &state.frames {
            write!(&mut msg, "\n").ok();
            self.fmt_source_loc(&mut msg, frame.begin - 1);
        }
        write!(&mut msg, "\n").ok();
        self.fmt_source_loc(&mut msg, state.pc);
        msg
    }

    fn source_loc(&self, pc: u64) -> Option<(u64, u64, u64)> {
        for (_, instr) in self.instrs().to_vec().iter().enumerate() {
            if let Instr::SourceLoc(other, id, line, col) = *instr {
                if other == pc {
                    return Some((id, line, col));
                }
            }
        }
        None
    }

    fn fmt_source_loc(&self, stream: &mut String, pc: usize) {
        let state = self.inner.state();
        match self.source_loc(pc as u64) {
            Some((i, line, col)) => {
                let token = &state.strings[&i];
                write!(stream, "> '{}' at line {} col {}", token, line, col).ok();
            }
            None => {
                let instr = self.inner.instrs()[pc];
                write!(stream, "> Unknown pc={} instr={:?}", pc, instr).ok();
            }
        }
    }
}

impl<S> Interpreter<S> for StackTraceInterpreter<S> {
    fn instrs(&self) -> &[Instr] {
        self.inner.instrs()
    }

    fn state(&self) -> InterpState {
        self.inner.state()
    }

    fn data_mut(&mut self) -> &mut S {
        self.inner.data_mut()
    }

    fn reset(&mut self) {
        self.inner.reset();
    }

    fn execute(&mut self, pc: usize, instr: Instr) -> InterpResult {
        self.inner.execute(pc, instr)
    }

    fn eval(&mut self, pc: usize) -> InterpResult {
        match self.inner.eval(pc) {
            Ok(val) => Ok(val),
            Err(err) => {
                let mut trace = self.stack_trace();
                if let Some(reason) = err.reason {
                    write!(&mut trace, "\n{}", reason).ok();
                }
                Err(Error::with(err.kind, &trace))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_variables() {
        let instrs = vec![
            Instr::LoadNumber(3.0),
            Instr::StoreVar(hash_str("foo")),
            Instr::LoadNumber(2.0),
            Instr::LoadVar(hash_str("foo")),
            Instr::Return,
        ];
        let mut interp = BaseInterpreter::new(instrs, &HashMap::new(), ());
        let res = interp.eval(1).unwrap();
        assert_eq!(res.unwrap(), Value::Number(3.0));
    }

    #[test]
    fn test_block_zero() {
        let instrs = vec![
            Instr::Begin(hash_str("main")),
            Instr::LoadVar(1337),
            Instr::Return,
            Instr::End(hash_str("main")),
            Instr::Begin(0),
            Instr::LoadNumber(200.0),
            Instr::StoreGlob(1337),
            Instr::Return,
            Instr::End(0),
        ];
        let mut interp = BaseInterpreter::new(instrs, &HashMap::new(), ());
        let res = interp.eval(1).unwrap();
        assert_eq!(res.unwrap(), Value::Number(200.0));
    }

    #[test]
    fn test_store_strings() {
        let instrs = vec![
            Instr::Begin(hash_str("main")),
            Instr::LoadString(0),
            Instr::Return,
            Instr::End(hash_str("main")),
            Instr::Begin(0),
            Instr::StoreString(0, 3),
            Instr::RawData(97),
            Instr::RawData(98),
            Instr::RawData(99),
            Instr::Return,
            Instr::End(0),
        ];
        let mut interp = BaseInterpreter::new(instrs, &HashMap::new(), ());
        let res = interp.eval(1).unwrap();
        assert_eq!(res.unwrap(), Value::Str(String::from("abc")));
    }
}
