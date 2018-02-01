use std::fmt;

use err::AssemErr;


#[derive(Copy, Clone, Debug, PartialEq, Serialize)]
pub enum Value<'a> {
    Variable(&'a str),
    Number(f64),
    Symbol(&'a str),
    Keyword(&'a str),
    StringLiteral(&'a str),
}

#[derive(Copy, Clone, Debug, PartialEq, Serialize)]
pub enum Symbol<'a> {
    ListBegin,
    ListEnd,
    ExpBegin,
    ExpEnd,
    GroupBegin,
    GroupEnd,
    Null,
    Assign(&'a str),
}

#[derive(Copy, Clone, Debug, PartialEq, Serialize)]
pub enum Argument<'a> {
    Arg(Token<Value<'a>>),
    Kwarg(Token<&'a str>, Token<Value<'a>>),
}

#[derive(Copy, Clone, Debug, PartialEq, Serialize)]
pub enum Code<'a> {
    Symbol(Symbol<'a>),
    Value(Value<'a>),
}

#[derive(Copy, Clone, Debug, PartialEq, Serialize)]
pub enum Name {
    Version,
    Globals,
    Def,
    Track,
}

#[derive(Copy, Clone, Debug, PartialEq, Serialize)]
pub struct Location {
    pub line: usize,
    pub col: usize,
    pub begin: usize,
    pub end: usize,
}

impl Location {
    pub fn new(line: usize, col: usize, begin: usize, end: usize) -> Location {
        Location {
            line: line,
            col: col,
            begin: begin,
            end: end,
        }
    }
}

impl Default for Location {
    fn default() -> Location {
        Location::new(1, 0, 0, 0)
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Serialize)]
pub struct Token<T> {
    pub loc: Location,
    pub data: T,
}

impl<T> Token<T>
where
    T: Copy,
{
    pub fn new(data: T, pos: Location) -> Token<T> {
        Token {
            data: data,
            loc: pos,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Directive<'a> {
    pub name: Token<Name>,
    pub args: Vec<Argument<'a>>,
    pub body: Vec<Token<Code<'a>>>,
}

impl<'a> Directive<'a> {
    pub fn arg_at(&self, idx: usize) -> Result<Argument, AssemErr> {
        match self.args.get(idx) {
            Some(arg) => Ok(*arg),
            None => Err(AssemErr::DuplicateVariable),
        }
    }
}

impl fmt::Display for Name {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Name::Version => write!(f, ".version"),
            Name::Def => write!(f, ".def"),
            Name::Globals => write!(f, ".globals"),
            Name::Track => write!(f, ".track"),
        }
    }
}

impl<'a> fmt::Display for Value<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Value::Variable(var) => write!(f, "@{}", var),
            Value::Number(num) => write!(f, "{}", num),
            Value::Symbol(sym) => write!(f, "'{}", sym),
            Value::Keyword(word) => write!(f, "{}", word),
            Value::StringLiteral(lit) => write!(f, "\"{}\"", lit),
        }
    }
}

impl<'a> fmt::Display for Symbol<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Symbol::ListBegin => write!(f, "["),
            Symbol::ListEnd => write!(f, "]"),
            Symbol::ExpBegin => write!(f, "("),
            Symbol::ExpEnd => write!(f, ")"),
            Symbol::GroupBegin => write!(f, "{{"),
            Symbol::GroupEnd => write!(f, "}}"),
            Symbol::Null => write!(f, "~"),
            Symbol::Assign(var) => write!(f, "= @{}", var),
        }
    }
}

impl<'a> fmt::Display for Code<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Code::Symbol(sym) => write!(f, "{}", sym),
            Code::Value(val) => write!(f, "{}", val),
        }
    }
}

impl<'a> fmt::Display for Argument<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Argument::Arg(val) => write!(f, "{}", val.data),
            Argument::Kwarg(key, val) => {
                write!(f, "@{} = {}", key.data, val.data)
            }
        }
    }
}

impl<'a> fmt::Display for Directive<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        try!(write!(f, "{}", self.name.data));

        for arg in &self.args {
            try!(write!(f, " "));
            try!(write!(f, "{}", arg));
        }

        if self.body.len() != 0 {
            try!(write!(f, ":\n "));
        }

        for code in &self.body {
            try!(write!(f, " "));
            try!(write!(f, "{}", code.data));
        }

        write!(f, "\n")
    }
}

impl<'a> Value<'a> {
    pub fn as_num(&self) -> Result<f64, AssemErr> {
        match *self {
            Value::Number(num) => Ok(num),
            _ => Err(AssemErr::DuplicateVariable),
        }
    }

    pub fn as_keyword(&self) -> Result<&str, AssemErr> {
        match *self {
            Value::Keyword(word) => Ok(word),
            _ => Err(AssemErr::DuplicateVariable),
        }
    }
}

impl<'a> Argument<'a> {
    pub fn as_value(&self) -> Result<Value<'a>, AssemErr> {
        match *self {
            Argument::Arg(ref val) => Ok(val.data),
            _ => Err(AssemErr::DuplicateVariable),
        }
    }
}
