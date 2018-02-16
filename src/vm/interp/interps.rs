use std::collections::HashMap;

use err::RuntimeErr;
use lang::hash_str;

pub use super::types::{Instr, InterpResult, Value};
pub use super::stack::StackFrame;
pub use super::state::InterpState;

pub type Keyword<S> = fn(&mut S, &mut InterpState) -> InterpResult;

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

pub struct Interpreter<S> {
    pub data: S,
    pub state: InterpState,
    instrs: Vec<Instr>,
    words: HashMap<u64, Keyword<S>>,
    strings: HashMap<u64, String>,
}

impl<S> Interpreter<S> {
    pub fn new(
        instrs: Vec<Instr>,
        exts: &HashMap<&'static str, Keyword<S>>,
        data: S,
    ) -> Interpreter<S> {
        let mut words = HashMap::new();
        for (word, func) in exts {
            words.insert(hash_str(word), *func);
        }

        let mut interpreter = Interpreter {
            instrs: instrs,
            words: words,
            data: data,
            state: InterpState::new(),
            strings: HashMap::new(),
        };

        interpreter.eval_block(0).ok();
        interpreter.state.reserved = interpreter.state.heap.len();
        interpreter.state.reset();
        interpreter
    }

    pub fn step(&mut self, instr: Instr) -> InterpResult {
        match instr {
            Instr::Null => self.state.push(Value::Null),
            Instr::LoadNumber(n) => self.state.push(Value::Number(n)),
            Instr::LoadSymbol(s) => self.state.push(Value::Symbol(s)),

            Instr::Call(args, pc) => self.state.call(args, pc),
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
                    Err(RuntimeErr::UnknownKeyword(word))
                }
            }

            Instr::LoadString(id) => {
                // Look up the string and push to the stack
                match self.strings.get(&id) {
                    Some(string) => {
                        try!(self.state.push(Value::Str(string.clone())));
                        Ok(None)
                    }
                    None => Err(RuntimeErr::InvalidArgs),
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
                        _ => return Err(RuntimeErr::InvalidString),
                    };
                }
                match String::from_utf8(bytes) {
                    Ok(string) => self.strings.insert(id, string),
                    Err(_) => return Err(RuntimeErr::InvalidString),
                };
                self.state.pc += len as usize;
                Ok(None)
            }

            _ => Ok(None),
        }
    }

    pub fn eval(&mut self, pc: usize) -> InterpResult {
        try!(self.state.call(0, pc));
        while self.state.pc < self.instrs.len() && !self.state.exit {
            let instr = self.instrs[self.state.pc];
            match try!(self.step(instr)) {
                None => (),
                Some(val) => return Ok(Some(val)),
            }
            self.state.pc += 1;
        }
        Ok(None)
    }

    pub fn eval_block(&mut self, block: u64) -> InterpResult {
        let instrs = self.instrs.clone();
        for (pc, instr) in instrs.iter().enumerate() {
            if let Instr::Begin(word) = *instr {
                if word == block {
                    return self.eval(pc + 1);
                }
            }
        }
        Err(RuntimeErr::InvalidArgs)
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
        let mut interp = Interpreter::new(instrs, &HashMap::new(), ());
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
        let mut interp = Interpreter::new(instrs, &HashMap::new(), ());
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
        let mut interp = Interpreter::new(instrs, &HashMap::new(), ());
        let res = interp.eval(1).unwrap();
        assert_eq!(res.unwrap(), Value::Str(String::from("abc")));
    }
}
