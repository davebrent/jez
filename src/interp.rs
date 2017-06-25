use std::collections::HashMap;
use std::convert::Into;

use err::RuntimeErr;
use lang::{hash_str, Instr};
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

pub type InterpResult = Result<(), RuntimeErr>;

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

/// Drop the top value on the stack
pub fn drop(state: &mut InterpState) -> InterpResult {
    state.stack.pop().unwrap();
    Ok(())
}

/// Duplicate the top value on the stack
pub fn duplicate(state: &mut InterpState) -> InterpResult {
    let val = *state.stack.last().unwrap();
    state.stack.push(val);
    Ok(())
}

/// Swap the top two values on the stack
pub fn swap(state: &mut InterpState) -> InterpResult {
    let a = state.stack.pop().unwrap();
    let b = state.stack.pop().unwrap();
    state.stack.push(a);
    state.stack.push(b);
    Ok(())
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
    words: HashMap<u64, Keyword<S>>,
}

impl<S> Interpreter<S> {
    pub fn new(exts: HashMap<&'static str, ExtKeyword<S>>,
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

        Interpreter {
            words: words,
            data: data,
            state: InterpState::new(),
        }
    }

    pub fn eval(&mut self, instrs: &[Instr]) -> InterpResult {
        self.state.pc = 0;
        while self.state.pc < instrs.len() {
            let instr = instrs[self.state.pc];
            match instr {
                Instr::LoadNumber(num) => {
                    load_number(num as f64, &mut self.state)
                }
                Instr::LoadString(_) => Err(RuntimeErr::NotImplemented),
                Instr::Null => load_null(&mut self.state),
                Instr::LoadSymbol(sym) => load_symbol(sym, &mut self.state),
                Instr::StoreVar(name) => store_var(name, &mut self.state),
                Instr::LoadVar(name) => load_var(name, &mut self.state),
                Instr::ListBegin => list_begin(&mut self.state),
                Instr::ListEnd => list_end(&mut self.state),
                Instr::Keyword(word) => {
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
            }?;
            self.state.pc += 1;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;


    #[test]
    fn test_addition() {
        let instrs = [Instr::LoadNumber(3.2),
                      Instr::LoadNumber(2.8),
                      Instr::Keyword(hash_str("add"))];
        let mut interp = Interpreter::new(HashMap::new(), ());
        interp.eval(&instrs).unwrap();
        let val: Option<f64> = interp.state.stack.pop().unwrap().into();
        let val = val.unwrap();
        assert_eq!(val, 6.0);
    }

    #[test]
    fn test_subtraction() {
        let instrs = [Instr::LoadNumber(2.0),
                      Instr::LoadNumber(3.0),
                      Instr::Keyword(hash_str("subtract"))];
        let mut interp = Interpreter::new(HashMap::new(), ());
        interp.eval(&instrs).unwrap();
        let val: Option<f64> = interp.state.stack.pop().unwrap().into();
        let val = val.unwrap();
        assert_eq!(val, -1.0);
    }

    #[test]
    fn test_variables() {
        let instrs = [Instr::LoadNumber(3.0),
                      Instr::StoreVar(hash_str("foo")),
                      Instr::LoadNumber(2.0),
                      Instr::LoadVar(hash_str("foo"))];
        let mut interp = Interpreter::new(HashMap::new(), ());
        interp.eval(&instrs).unwrap();
        let val: Option<f64> = interp.state.stack.pop().unwrap().into();
        let val = val.unwrap();
        assert_eq!(val, 3.0);
    }

    #[test]
    fn test_swap() {
        let instrs = [Instr::LoadNumber(3.0),
                      Instr::LoadNumber(2.0),
                      Instr::Keyword(hash_str("swap"))];
        let mut interp = Interpreter::new(HashMap::new(), ());
        interp.eval(&instrs).unwrap();
        let a: Option<f64> = interp.state.stack.pop().unwrap().into();
        let a = a.unwrap();
        assert_eq!(a, 3.0);
        let b: Option<f64> = interp.state.stack.pop().unwrap().into();
        let b = b.unwrap();
        assert_eq!(b, 2.0);
    }
}
