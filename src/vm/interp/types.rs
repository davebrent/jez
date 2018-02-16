use err::RuntimeErr;

use vm::math::Curve;

pub type InterpResult = Result<Option<Value>, RuntimeErr>;

#[derive(Copy, Clone, Debug, PartialEq, Serialize)]
pub enum Instr {
    Begin(u64),
    End(u64),
    Call(usize, usize),
    Return,
    LoadNumber(f64),
    LoadSymbol(u64),
    LoadVar(u64),
    LoadString(u64),
    StoreString(u64, u64),
    RawData(u8),
    StoreGlob(u64),
    StoreVar(u64),
    Keyword(u64),
    ListBegin,
    ListEnd,
    SeqBegin,
    SeqEnd,
    GroupBegin,
    GroupEnd,
    Null,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub enum Value {
    Null,
    Number(f64),
    Symbol(u64),
    List(usize, usize),
    Group(usize, usize),
    Seq(usize, usize),
    Str(String),
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

    pub fn as_range(&self) -> Result<(usize, usize), RuntimeErr> {
        match *self {
            Value::List(a, b) | Value::Group(a, b) | Value::Seq(a, b) => Ok((a, b)),
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
