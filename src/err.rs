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
pub enum AssemErr {
    UnsupportedVersion(u64),
    DuplicateVariable,
    DuplicateFunction,
}

impl Error for AssemErr {
    fn description(&self) -> &str {
        match *self {
            AssemErr::UnsupportedVersion(_) => "unsupported version",
            AssemErr::DuplicateVariable => "duplicate variable",
            AssemErr::DuplicateFunction => "duplicate function",
        }
    }

    fn cause(&self) -> Option<&Error> {
        None
    }
}

impl fmt::Display for AssemErr {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            AssemErr::UnsupportedVersion(req) => {
                write!(f, "unsupported version, requires '{}'", req)
            }
            AssemErr::DuplicateVariable => write!(f, "duplicate variable"),
            AssemErr::DuplicateFunction => write!(f, "duplicate function"),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize)]
pub enum ParseErr {
    Incomplete(usize, usize),
    UnexpectedToken(usize, usize),
}

impl Error for ParseErr {
    fn description(&self) -> &str {
        match *self {
            ParseErr::Incomplete(_, _) => "incomplete token",
            ParseErr::UnexpectedToken(_, _) => "unknown token",
        }
    }

    fn cause(&self) -> Option<&Error> {
        None
    }
}

impl fmt::Display for ParseErr {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            ParseErr::Incomplete(line, col) => {
                write!(f, "incomplete token on line {} col {}", line, col)
            }
            ParseErr::UnexpectedToken(line, col) => {
                write!(f, "unknown token on line {} col {}", line, col)
            }
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize)]
pub enum RuntimeErr {
    UnknownKeyword(u64),
    InvalidArgs,
    InvalidString,
    StackExhausted,
}

impl Error for RuntimeErr {
    fn description(&self) -> &str {
        match *self {
            RuntimeErr::UnknownKeyword(_) => "unknown keyword",
            RuntimeErr::InvalidArgs => "invalid arguments",
            RuntimeErr::InvalidString => "invalid string",
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
            RuntimeErr::InvalidArgs => write!(f, "invalid arguments"),
            RuntimeErr::InvalidString => write!(f, "invalid string"),
            RuntimeErr::StackExhausted => write!(f, "stack exhausted"),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize)]
pub enum JezErr {
    AssemErr(AssemErr),
    ParseErr(ParseErr),
    RuntimeErr(RuntimeErr),
    SysErr(SysErr),
    IoErr,
}

impl From<AssemErr> for JezErr {
    fn from(err: AssemErr) -> JezErr {
        JezErr::AssemErr(err)
    }
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
        JezErr::IoErr
    }
}

impl Error for JezErr {
    fn description(&self) -> &str {
        match *self {
            JezErr::AssemErr(ref err) => err.description(),
            JezErr::ParseErr(ref err) => err.description(),
            JezErr::RuntimeErr(ref err) => err.description(),
            JezErr::SysErr(ref err) => err.description(),
            JezErr::IoErr => "io error",
        }
    }

    fn cause(&self) -> Option<&Error> {
        match *self {
            JezErr::AssemErr(ref err) => Some(err as &Error),
            JezErr::ParseErr(ref err) => Some(err as &Error),
            JezErr::RuntimeErr(ref err) => Some(err as &Error),
            JezErr::SysErr(ref err) => Some(err as &Error),
            JezErr::IoErr => None,
        }
    }
}

impl fmt::Display for JezErr {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            JezErr::AssemErr(ref err) => write!(f, "Assembly error {}", err),
            JezErr::ParseErr(ref err) => write!(f, "Parse error {}", err),
            JezErr::RuntimeErr(ref err) => write!(f, "Runtime error {}", err),
            JezErr::SysErr(ref err) => write!(f, "System error {}", err),
            JezErr::IoErr => write!(f, "IO error"),
        }
    }
}
