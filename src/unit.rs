//! # Units
//!
//! Units are the building block of the virtual machine, responsible for
//! executing instructions and performing domain specific operations.
//!
//! Each unit operates independently from others, maintaining their own "heaps"
//! and communicate only via message passsing.
//!
//! This module contains the shared functionality common across the units.

use std::collections::HashMap;
use std::convert::Into;
use std::fmt;

use lang::Instr;


/// Represents all the values possible that can go on the stack
#[derive(Copy, Clone, Debug)]
pub enum Value {
    /// All numbers are floats
    Number(f32),
    /// A hashed string
    Symbol(u64),
    /// Used for representing lists, a range of values on the heap
    Pair(usize, usize),
    /// Interpreter instructions
    Instruction(Instr),
    /// Special value of nothing
    Null,
}

impl Into<Option<f32>> for Value {
    fn into(self) -> Option<f32> {
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

#[derive(Clone, Copy, Debug)]
pub enum RuntimeErr {
    UnknownKeyword(u64),
    UnmatchedPair,
    NotImplemented,
    WrongType,
    InvalidArguments,
}

impl fmt::Display for RuntimeErr {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            RuntimeErr::UnknownKeyword(word) => {
                write!(f, "Unknown keyword {}", word)
            }
            RuntimeErr::UnmatchedPair => {
                write!(f, "Unmatched pair, stack exhausted")
            }
            RuntimeErr::NotImplemented => {
                write!(f, "Instruction is not yet implemented")
            }
            RuntimeErr::WrongType => {
                write!(f, "Function received the wrong type")
            }
            RuntimeErr::InvalidArguments => {
                write!(f, "Function received wrong arguments")
            }
        }
    }
}

/// Inter-unit messages
#[derive(Clone, Copy, Debug)]
pub enum Message {
    /// Sent from spu
    TriggerEvent(f32, f32),
    /// Sent from units to the machine
    HasError(u8, RuntimeErr),
    /// Sent from the machine to units, used for reloading
    Stop,
}

pub type InterpResult = Result<(), RuntimeErr>;
pub type Keyword = fn(&mut InterpState) -> InterpResult;

/// Basic interpreter state
#[derive(Debug)]
pub struct InterpState {
    /// Program counter, into a slice of instructions
    pub pc: usize,
    /// Main stack for computation
    pub stack: Vec<Value>,
    /// Second stack OR heap memory
    pub heap: Vec<Value>,
    /// Variable mapping table, points into `heap` by default
    pub vars: HashMap<u64, usize>,
}

impl InterpState {
    pub fn new() -> InterpState {
        InterpState {
            pc: 0,
            stack: vec![],
            heap: vec![],
            vars: HashMap::new(),
        }
    }
}

/// Put the Null value onto the stack
pub fn load_null(state: &mut InterpState) -> InterpResult {
    state.stack.push(Value::Null);
    Ok(())
}

/// Put a number onto the stack
pub fn load_number(num: f32, state: &mut InterpState) -> InterpResult {
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
    let ptr = state.vars.get(&name).unwrap();
    state.stack.push(state.heap.get(*ptr).unwrap().clone());
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
            None => return Err(RuntimeErr::UnmatchedPair),
            Some(val) => {
                match val {
                    Value::Instruction(instr) => {
                        match instr {
                            Instr::ListBegin => {
                                let end = state.heap.len();
                                let pair = Value::Pair(start, end);
                                &state.heap[start..end].reverse();
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
    let rhs: Option<f32> = state.stack.pop().unwrap().into();
    let lhs: Option<f32> = state.stack.pop().unwrap().into();
    state
        .stack
        .push(Value::Number(lhs.unwrap() + rhs.unwrap()));
    Ok(())
}

/// Subtract top two stack values
pub fn subtract(state: &mut InterpState) -> InterpResult {
    let rhs: Option<f32> = state.stack.pop().unwrap().into();
    let lhs: Option<f32> = state.stack.pop().unwrap().into();
    state
        .stack
        .push(Value::Number(lhs.unwrap() - rhs.unwrap()));
    Ok(())
}

/// Multiply top two stack values
pub fn multiply(state: &mut InterpState) -> InterpResult {
    let rhs: Option<f32> = state.stack.pop().unwrap().into();
    let lhs: Option<f32> = state.stack.pop().unwrap().into();
    state
        .stack
        .push(Value::Number(lhs.unwrap() * rhs.unwrap()));
    Ok(())
}

/// Divide top two stack values
pub fn divide(state: &mut InterpState) -> InterpResult {
    let rhs: Option<f32> = state.stack.pop().unwrap().into();
    let lhs: Option<f32> = state.stack.pop().unwrap().into();
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
        let instr = instrs.get(state.pc).unwrap();
        match *instr {
            Instr::LoadNumber(num) => load_number(num, state),
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
    return Ok(());
}
