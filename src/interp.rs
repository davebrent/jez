use std::collections::HashMap;

use assem::hash_str;
use err::RuntimeErr;
use math::Curve;


#[derive(Copy, Clone, Debug, PartialEq, Serialize)]
pub enum Instr {
    Begin(u64),
    End(u64),
    Call(usize, usize),
    Return,
    LoadNumber(f64),
    LoadSymbol(u64),
    LoadVar(u64),
    StoreGlob(u64),
    StoreVar(u64),
    Keyword(u64),
    ListBegin,
    ListEnd,
    Null,
}

#[derive(Copy, Clone, Debug, PartialEq, Serialize)]
pub enum Value {
    Null,
    Number(f64),
    Symbol(u64),
    Pair(usize, usize),
    Tuple(usize, usize),
    Instruction(Instr),
    Curve(Curve),
}

impl Value {
    pub fn as_num(&self) -> Result<f64, RuntimeErr> {
        match *self {
            Value::Number(num) => Ok(num),
            _ => Err(RuntimeErr::InvalidArgs),
        }
    }

    pub fn as_sym(&self) -> Result<u64, RuntimeErr> {
        match *self {
            Value::Symbol(sym) => Ok(sym),
            _ => Err(RuntimeErr::InvalidArgs),
        }
    }
}

pub type InterpResult = Result<Option<Value>, RuntimeErr>;

#[derive(Debug)]
struct StackFrame {
    stack: Vec<Value>,
    locals: HashMap<u64, usize>,
    ret_addr: usize,
}

impl StackFrame {
    pub fn new(ret_addr: usize) -> StackFrame {
        StackFrame {
            stack: Vec::new(),
            ret_addr: ret_addr,
            locals: HashMap::new(),
        }
    }

    pub fn last(&self) -> Result<Value, RuntimeErr> {
        match self.stack.last() {
            Some(val) => Ok(*val),
            None => Err(RuntimeErr::StackExhausted),
        }
    }

    pub fn pop(&mut self) -> Result<Value, RuntimeErr> {
        match self.stack.pop() {
            Some(val) => Ok(val),
            None => Err(RuntimeErr::StackExhausted),
        }
    }

    pub fn push(&mut self, val: Value) -> Result<(), RuntimeErr> {
        self.stack.push(val);
        Ok(())
    }
}

#[derive(Debug)]
pub struct InterpState {
    reserved: usize,
    heap: Vec<Value>,
    pc: usize,
    globals: HashMap<u64, usize>,
    frames: Vec<StackFrame>,
    exit: bool,
}

impl InterpState {
    pub fn new() -> InterpState {
        InterpState {
            pc: 0,
            reserved: 0,
            heap: vec![],
            globals: HashMap::new(),
            frames: vec![],
            exit: false,
        }
    }

    fn frame(&self) -> Result<&StackFrame, RuntimeErr> {
        match self.frames.last() {
            None => Err(RuntimeErr::StackExhausted),
            Some(frame) => Ok(frame),
        }
    }

    fn frame_mut(&mut self) -> Result<&mut StackFrame, RuntimeErr> {
        match self.frames.last_mut() {
            None => Err(RuntimeErr::StackExhausted),
            Some(frame) => Ok(frame),
        }
    }

    pub fn heap_slice_mut(&mut self,
                          start: usize,
                          end: usize)
                          -> Result<&mut [Value], RuntimeErr> {
        if start > end || end > self.heap_len() {
            return Err(RuntimeErr::InvalidArgs);
        }
        Ok(&mut self.heap[start..end])
    }

    pub fn heap_get(&self, ptr: usize) -> Result<Value, RuntimeErr> {
        match self.heap.get(ptr) {
            Some(val) => Ok(*val),
            None => Err(RuntimeErr::InvalidArgs),
        }
    }

    pub fn heap_len(&self) -> usize {
        self.heap.len()
    }

    pub fn heap_push(&mut self, val: Value) -> usize {
        self.heap.push(val);
        self.heap_len()
    }

    pub fn call(&mut self, args: usize, pc: usize) -> InterpResult {
        // Push a new stack frame copying across any arguments, if any, from
        // the previous frame
        let mut frame = StackFrame::new(self.pc);
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
            None => Err(RuntimeErr::StackExhausted),
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
        let mut frame = try!(self.frame_mut());
        Ok(try!(frame.pop()))
    }

    pub fn pop_num(&mut self) -> Result<f64, RuntimeErr> {
        match try!(self.pop()) {
            Value::Number(num) => Ok(num),
            _ => Err(RuntimeErr::InvalidArgs),
        }
    }

    pub fn pop_pair(&mut self) -> Result<(usize, usize), RuntimeErr> {
        match try!(self.pop()) {
            Value::Pair(start, end) => Ok((start, end)),
            _ => Err(RuntimeErr::InvalidArgs),
        }
    }

    pub fn last_pair(&mut self) -> Result<(usize, usize), RuntimeErr> {
        match try!(self.last()) {
            Value::Pair(start, end) => Ok((start, end)),
            _ => Err(RuntimeErr::InvalidArgs),
        }
    }

    pub fn push(&mut self, val: Value) -> InterpResult {
        match self.frames.last_mut() {
            None => Err(RuntimeErr::StackExhausted),
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

    pub fn store_glob(&mut self,
                      name: u64,
                      val: Value)
                      -> Result<(), RuntimeErr> {
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
            None => Err(RuntimeErr::InvalidArgs),
        }
    }

    pub fn reset(&mut self) {
        self.exit = false;
        self.heap.truncate(self.reserved);;
    }
}

fn add(state: &mut InterpState) -> InterpResult {
    let rhs = try!(state.pop_num());
    let lhs = try!(state.pop_num());
    try!(state.push(Value::Number(lhs + rhs)));
    Ok(None)
}

fn subtract(state: &mut InterpState) -> InterpResult {
    let rhs = try!(state.pop_num());
    let lhs = try!(state.pop_num());
    try!(state.push(Value::Number(lhs - rhs)));
    Ok(None)
}

fn multiply(state: &mut InterpState) -> InterpResult {
    let rhs = try!(state.pop_num());
    let lhs = try!(state.pop_num());
    try!(state.push(Value::Number(lhs * rhs)));
    Ok(None)
}

fn divide(state: &mut InterpState) -> InterpResult {
    let rhs = try!(state.pop_num());
    let lhs = try!(state.pop_num());
    try!(state.push(Value::Number(lhs / rhs)));
    Ok(None)
}

fn print(state: &mut InterpState) -> InterpResult {
    let val = try!(state.last());
    println!("{:?}", val);
    Ok(None)
}

fn drop(state: &mut InterpState) -> InterpResult {
    try!(state.pop());
    Ok(None)
}

fn duplicate(state: &mut InterpState) -> InterpResult {
    let val = try!(state.last());
    try!(state.push(val));
    Ok(None)
}

fn swap(state: &mut InterpState) -> InterpResult {
    let a = try!(state.pop());
    let b = try!(state.pop());
    try!(state.push(a));
    try!(state.push(b));
    Ok(None)
}

pub type BuiltInKeyword = fn(&mut InterpState) -> InterpResult;
pub type ExtKeyword<S> = fn(&mut S, &mut InterpState) -> InterpResult;

pub enum Keyword<S> {
    BuiltIn(BuiltInKeyword),
    Extension(ExtKeyword<S>),
}

pub struct Interpreter<S> {
    pub data: S,
    pub state: InterpState,
    instrs: Vec<Instr>,
    words: HashMap<u64, Keyword<S>>,
}

impl<S> Interpreter<S> {
    pub fn new(instrs: Vec<Instr>,
               exts: HashMap<&'static str, ExtKeyword<S>>,
               data: S)
               -> Interpreter<S> {
        let mut words = HashMap::new();
        words.insert(hash_str("add"), Keyword::BuiltIn(add));
        words.insert(hash_str("divide"), Keyword::BuiltIn(divide));
        words.insert(hash_str("multiply"), Keyword::BuiltIn(multiply));
        words.insert(hash_str("print"), Keyword::BuiltIn(print));
        words.insert(hash_str("subtract"), Keyword::BuiltIn(subtract));
        words.insert(hash_str("drop"), Keyword::BuiltIn(drop));
        words.insert(hash_str("dup"), Keyword::BuiltIn(duplicate));
        words.insert(hash_str("swap"), Keyword::BuiltIn(swap));

        for (word, func) in &exts {
            words.insert(hash_str(word), Keyword::Extension(*func));
        }

        let instrs_len = instrs.len();
        let mut inner_main = instrs.len();
        for (pc, instr) in instrs.iter().enumerate() {
            if let Instr::Begin(word) = *instr {
                if word == 0 {
                    inner_main = pc + 1;
                    break;
                }
            }
        }

        let mut interpreter = Interpreter {
            instrs: instrs,
            words: words,
            data: data,
            state: InterpState::new(),
        };

        // This block is created by the assembler so must always succeed
        if inner_main != instrs_len {
            interpreter.eval(inner_main).unwrap();
            interpreter.state.reserved = interpreter.state.heap.len();
            interpreter.state.reset();
        }

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
            Instr::ListBegin => {
                let val = Value::Instruction(Instr::ListBegin);
                try!(self.state.push(val));
                Ok(None)
            }
            Instr::ListEnd => {
                let start = self.state.heap_len();
                loop {
                    // Loop back through the stack, moving objects to the heap,
                    // until a 'ListBegin' instruction is reached then, store
                    // the range on the stack
                    let val = try!(self.state.pop());
                    match val {
                        Value::Instruction(instr) => {
                            match instr {
                                Instr::ListBegin => {
                                    let end = self.state.heap_len();
                                    let pair = Value::Pair(start, end);
                                    try!(self.state.push(pair));
                                    try!(self.state.heap_slice_mut(start, end))
                                        .reverse();
                                    return Ok(None);
                                }
                                _ => {
                                    self.state.heap_push(val);
                                }
                            }
                        }
                        _ => {
                            self.state.heap_push(val);
                        }
                    }
                }
            }
            Instr::Keyword(word) => {
                // Keywords operate on an implicit stack frame
                if let Some(keyword) = self.words.get(&word) {
                    match *keyword {
                        Keyword::BuiltIn(func) => func(&mut self.state),
                        Keyword::Extension(func) => {
                            func(&mut self.data, &mut self.state)
                        }
                    }
                } else {
                    Err(RuntimeErr::UnknownKeyword(word))
                }
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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_callables() {
        let instrs = vec![Instr::Begin(1),
                          Instr::LoadNumber(13.0),
                          Instr::LoadNumber(12.0),
                          Instr::Call(2, 6),
                          Instr::Return,
                          Instr::End(1),
                          Instr::Begin(2),
                          Instr::Keyword(hash_str("add")),
                          Instr::Return,
                          Instr::End(2)];
        let mut interp = Interpreter::new(instrs, HashMap::new(), ());
        let res = interp.eval(1).unwrap();
        assert_eq!(res.unwrap(), Value::Number(25.0));
    }

    #[test]
    fn test_addition() {
        let instrs = vec![Instr::LoadNumber(3.2),
                          Instr::LoadNumber(2.8),
                          Instr::Keyword(hash_str("add")),
                          Instr::Return];
        let mut interp = Interpreter::new(instrs, HashMap::new(), ());
        let res = interp.eval(1).unwrap();
        assert_eq!(res.unwrap(), Value::Number(6.0));
    }

    #[test]
    fn test_subtraction() {
        let instrs = vec![Instr::LoadNumber(2.0),
                          Instr::LoadNumber(3.0),
                          Instr::Keyword(hash_str("subtract")),
                          Instr::Return];
        let mut interp = Interpreter::new(instrs, HashMap::new(), ());
        let res = interp.eval(1).unwrap();
        assert_eq!(res.unwrap(), Value::Number(-1.0));
    }

    #[test]
    fn test_variables() {
        let instrs = vec![Instr::LoadNumber(3.0),
                          Instr::StoreVar(hash_str("foo")),
                          Instr::LoadNumber(2.0),
                          Instr::LoadVar(hash_str("foo")),
                          Instr::Return];
        let mut interp = Interpreter::new(instrs, HashMap::new(), ());
        let res = interp.eval(1).unwrap();
        assert_eq!(res.unwrap(), Value::Number(3.0));
    }

    #[test]
    fn test_swap() {
        let instrs = vec![Instr::LoadNumber(3.0),
                          Instr::LoadNumber(2.0),
                          Instr::Keyword(hash_str("swap")),
                          Instr::Return];
        let mut interp = Interpreter::new(instrs, HashMap::new(), ());
        let res = interp.eval(1).unwrap();
        assert_eq!(res.unwrap(), Value::Number(3.0));
    }

    #[test]
    fn test_block_zero() {
        let instrs = vec![Instr::Begin(hash_str("main")),
                          Instr::LoadVar(1337),
                          Instr::Return,
                          Instr::End(hash_str("main")),
                          Instr::Begin(0),
                          Instr::LoadNumber(200.0),
                          Instr::StoreGlob(1337),
                          Instr::Return,
                          Instr::End(0)];
        let mut interp = Interpreter::new(instrs, HashMap::new(), ());
        let res = interp.eval(1).unwrap();
        assert_eq!(res.unwrap(), Value::Number(200.0));
    }
}
