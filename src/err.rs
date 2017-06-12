use std::convert::From;
use std::error::Error;
use std::fmt;
use std::io;


#[derive(Clone, Copy, Debug, PartialEq, Serialize)]
pub enum SysErr {
    UnknownBackend,
    UnreachableBackend,
}

impl Error for SysErr {
    fn description(&self) -> &str {
        match *self {
            SysErr::UnknownBackend => "unknown backend",
            SysErr::UnreachableBackend => "unreachable backend",
        }
    }

    fn cause(&self) -> Option<&Error> {
        None
    }
}

impl fmt::Display for SysErr {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            SysErr::UnknownBackend => write!(f, "Unknown backend"),
            SysErr::UnreachableBackend => write!(f, "Unreachable backend"),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize)]
pub enum ParseErr {
    InvalidInput,
    UnknownToken(usize, usize),
    UnmatchedPair(usize, usize),
    UnknownVariable(usize, usize),
}

impl Error for ParseErr {
    fn description(&self) -> &str {
        match *self {
            ParseErr::InvalidInput => "invalid input",
            ParseErr::UnknownToken(_, _) => "unknown token",
            ParseErr::UnmatchedPair(_, _) => "unmatched pair",
            ParseErr::UnknownVariable(_, _) => "unknown variable",
        }
    }

    fn cause(&self) -> Option<&Error> {
        None
    }
}

impl fmt::Display for ParseErr {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            ParseErr::InvalidInput => write!(f, "invalid input"),
            ParseErr::UnknownToken(line, col) => {
                write!(f, "unknown token on line {} col {}", line, col)
            }
            ParseErr::UnmatchedPair(line, col) => {
                write!(f, "unmatched pair on line {} col {}", line, col)
            }
            ParseErr::UnknownVariable(line, col) => {
                write!(f, "unknown variable on line {} col {}", line, col)
            }
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize)]
pub enum RuntimeErr {
    UnknownKeyword(u64),
    NotImplemented,
    InvalidArgs,
    StackExhausted,
}

impl Error for RuntimeErr {
    fn description(&self) -> &str {
        match *self {
            RuntimeErr::UnknownKeyword(_) => "unknown keyword",
            RuntimeErr::NotImplemented => "not implemented",
            RuntimeErr::InvalidArgs => "invalid arguments",
            RuntimeErr::StackExhausted => "stack exhausted",
        }
    }

    fn cause(&self) -> Option<&Error> {
        None
    }
}

impl fmt::Display for RuntimeErr {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            RuntimeErr::UnknownKeyword(hash) => {
                write!(f, "encountered unknown keyword (hash = {})", hash)
            }
            RuntimeErr::NotImplemented => write!(f, "keyword not implemented"),
            RuntimeErr::InvalidArgs => write!(f, "invalid arguments"),
            RuntimeErr::StackExhausted => write!(f, "stack exhausted"),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize)]
pub enum JezErr {
    ParseErr(ParseErr),
    RuntimeErr(RuntimeErr),
    SysErr(SysErr),
}

impl From<ParseErr> for JezErr {
    fn from(err: ParseErr) -> JezErr {
        JezErr::ParseErr(err)
    }
}

impl From<RuntimeErr> for JezErr {
    fn from(err: RuntimeErr) -> JezErr {
        JezErr::RuntimeErr(err)
    }
}

impl From<SysErr> for JezErr {
    fn from(err: SysErr) -> JezErr {
        JezErr::SysErr(err)
    }
}

impl From<io::Error> for JezErr {
    fn from(_: io::Error) -> JezErr {
        JezErr::ParseErr(ParseErr::InvalidInput)
    }
}

impl Error for JezErr {
    fn description(&self) -> &str {
        match *self {
            JezErr::ParseErr(ref err) => err.description(),
            JezErr::RuntimeErr(ref err) => err.description(),
            JezErr::SysErr(ref err) => err.description(),
        }
    }

    fn cause(&self) -> Option<&Error> {
        match *self {
            JezErr::ParseErr(ref err) => Some(err as &Error),
            JezErr::RuntimeErr(ref err) => Some(err as &Error),
            JezErr::SysErr(ref err) => Some(err as &Error),
        }
    }
}

impl fmt::Display for JezErr {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            JezErr::ParseErr(ref err) => write!(f, "Parse error {}", err),
            JezErr::RuntimeErr(ref err) => write!(f, "Runtime error {}", err),
            JezErr::SysErr(ref err) => write!(f, "System error {}", err),
        }
    }
}
