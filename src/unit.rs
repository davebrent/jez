use std::collections::HashMap;
use std::convert::Into;
use std::thread;
use std::time::{Duration, Instant};

use err::{RuntimeErr, JezErr};
use lang::Instr;
use math::Curve;


/// Represents all the values possible that can go on the stack
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum Value {
    Number(f64),
    Symbol(u64),
    Pair(usize, usize),
    Tuple(usize, usize),
    Instruction(Instr),
    Null,
    Curve(Curve),
}

impl Into<Option<f64>> for Value {
    fn into(self) -> Option<f64> {
        match self {
            Value::Number(num) => Some(num),
            _ => None,
        }
    }
}

impl Into<Option<u64>> for Value {
    fn into(self) -> Option<u64> {
        match self {
            Value::Symbol(sym) => Some(sym),
            _ => None,
        }
    }
}

impl Into<Option<(usize, usize)>> for Value {
    fn into(self) -> Option<(usize, usize)> {
        match self {
            Value::Pair(a, b) => Some((a, b)),
            _ => None,
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Serialize)]
pub enum EventValue {
    Trigger(f64),
    Curve(Curve),
}

#[derive(Copy, Clone, Debug, PartialEq, Serialize)]
pub struct Event {
    pub track: u32,
    pub onset: f64,
    pub dur: f64,
    pub value: EventValue,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize)]
pub enum Message {
    Error(u8, JezErr),
    MidiCtl(u8, u8, u8),
    MidiNoteOff(u8, u8),
    MidiNoteOn(u8, u8, u8),
    SeqEvent(Event),
    Stop,
    Reload,
}

pub type InterpResult = Result<(), RuntimeErr>;
pub type Keyword = fn(&mut InterpState) -> InterpResult;

#[derive(Debug)]
pub struct InterpState {
    pub pc: usize,
    pub stack: Vec<Value>,
    pub heap: Vec<Value>,
    pub vars: HashMap<u64, usize>,
    save_point: usize,
}

impl InterpState {
    pub fn new() -> InterpState {
        InterpState {
            pc: 0,
            stack: vec![],
            heap: vec![],
            vars: HashMap::new(),
            save_point: 0,
        }
    }

    // /// All values pushed to the heap after this point may be truncated
    // pub fn set_save_point(&mut self) {
    //     self.save_point = self.heap.len();
    // }

    /// Remove all values from the heap above the save point
    pub fn reset(&mut self) {
        self.heap.truncate(self.save_point);
    }
}

/// Put the Null value onto the stack
pub fn load_null(state: &mut InterpState) -> InterpResult {
    state.stack.push(Value::Null);
    Ok(())
}

/// Put a number onto the stack
pub fn load_number(num: f64, state: &mut InterpState) -> InterpResult {
    state.stack.push(Value::Number(num));
    Ok(())
}

/// Puts a symbol value onto the stack
pub fn load_symbol(num: u64, state: &mut InterpState) -> InterpResult {
    state.stack.push(Value::Symbol(num));
    Ok(())
}

/// Move a value onto the heap and store it against a name
pub fn store_var(name: u64, state: &mut InterpState) -> InterpResult {
    let ptr = state.heap.len();
    let val = state.stack.pop().unwrap();
    state.heap.push(val);
    state.vars.insert(name, ptr);
    Ok(())
}

/// Load a variable value back onto the stack
pub fn load_var(name: u64, state: &mut InterpState) -> InterpResult {
    let ptr = &state.vars[&name];
    state.stack.push(state.heap[*ptr]);
    Ok(())
}

/// Marker for `list_end` to stop traversing the stack
pub fn list_begin(state: &mut InterpState) -> InterpResult {
    state.stack.push(Value::Instruction(Instr::ListBegin));
    Ok(())
}

/// Replaces stack values with a pair pointing to the values on the heap
pub fn list_end(state: &mut InterpState) -> InterpResult {
    let start = state.heap.len();
    loop {
        // Loop back through the stack, moving objects to the heap, until a
        // 'ListBegin' instruction is reached then, store the range on the stack
        match state.stack.pop() {
            None => return Err(RuntimeErr::StackExhausted),
            Some(val) => {
                match val {
                    Value::Instruction(instr) => {
                        match instr {
                            Instr::ListBegin => {
                                let end = state.heap.len();
                                let pair = Value::Pair(start, end);
                                state.heap[start..end].reverse();
                                state.stack.push(pair);
                                return Ok(());
                            }
                            _ => state.heap.push(val),
                        }
                    }
                    _ => state.heap.push(val),
                }
            }
        }
    }
}

/// Add top two stack values
pub fn add(state: &mut InterpState) -> InterpResult {
    let rhs: Option<f64> = state.stack.pop().unwrap().into();
    let lhs: Option<f64> = state.stack.pop().unwrap().into();
    state
        .stack
        .push(Value::Number(lhs.unwrap() + rhs.unwrap()));
    Ok(())
}

/// Subtract top two stack values
pub fn subtract(state: &mut InterpState) -> InterpResult {
    let rhs: Option<f64> = state.stack.pop().unwrap().into();
    let lhs: Option<f64> = state.stack.pop().unwrap().into();
    state
        .stack
        .push(Value::Number(lhs.unwrap() - rhs.unwrap()));
    Ok(())
}

/// Multiply top two stack values
pub fn multiply(state: &mut InterpState) -> InterpResult {
    let rhs: Option<f64> = state.stack.pop().unwrap().into();
    let lhs: Option<f64> = state.stack.pop().unwrap().into();
    state
        .stack
        .push(Value::Number(lhs.unwrap() * rhs.unwrap()));
    Ok(())
}

/// Divide top two stack values
pub fn divide(state: &mut InterpState) -> InterpResult {
    let rhs: Option<f64> = state.stack.pop().unwrap().into();
    let lhs: Option<f64> = state.stack.pop().unwrap().into();
    state
        .stack
        .push(Value::Number(lhs.unwrap() / rhs.unwrap()));
    Ok(())
}

/// Print top stack value
pub fn print(state: &mut InterpState) -> InterpResult {
    println!("{:?}", state.stack.pop().unwrap());
    Ok(())
}

/// An `Interpreter` provides a method for evaluating keywords
pub trait Interpreter {
    fn eval(&mut self, word: u64, state: &mut InterpState) -> InterpResult;
}

pub trait Unit {
    fn tick(&mut self, delta: &Duration) -> bool;

    fn run_forever(&mut self, res: Duration) {
        let mut previous = Instant::now();
        loop {
            let now = Instant::now();
            let delta = now.duration_since(previous);
            if self.tick(&delta) {
                return;
            }
            previous = now;
            thread::sleep(res);
        }
    }
}

/// Evaluate instructions, calling back to an `Interpreter` for evaling keywords
///
/// The `InterpState`'s program counter is updated automatically upon each
/// iteration, any keywords that manipulate it therefore have to take this into
/// account.
pub fn eval<T>(instrs: &[Instr],
               state: &mut InterpState,
               interp: &mut T)
               -> InterpResult
    where T: Interpreter
{
    state.pc = 0;
    while state.pc < instrs.len() {
        let instr = instrs[state.pc];
        match instr {
            Instr::LoadNumber(num) => load_number(num as f64, state),
            Instr::LoadString(_) => Err(RuntimeErr::NotImplemented),
            Instr::Null => load_null(state),
            Instr::LoadSymbol(sym) => load_symbol(sym, state),
            Instr::StoreVar(name) => store_var(name, state),
            Instr::LoadVar(name) => load_var(name, state),
            Instr::ListBegin => list_begin(state),
            Instr::ListEnd => list_end(state),
            Instr::Keyword(word) => interp.eval(word, state),
        }?;
        state.pc += 1;
    }
    Ok(())
}
