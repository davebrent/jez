use std::convert::From;
use std::error;
use std::fmt;
use std::fmt::Write;
use std::io;

use serde::Serialize;

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Location {
    pub filename: &'static str,
    pub line: u32,
    pub column: u32,
}

impl fmt::Display for Location {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let filename = self.filename;
        write!(
            f,
            "file '{}' at line {} column {}",
            filename, self.line, self.column
        )
    }
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub enum Kind {
    Internal(Location),
    UnreachableBackend,
    UnknownBackend,
    UnsupportedVersion,
    DuplicateVariable,
    DuplicateFunction,
    IncompleteInput,
    UnexpectedToken,
    UnknownKeyword,
    StackExhausted,
    InvalidArgs,
    Io,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Error {
    pub kind: Kind,
    pub reason: Option<String>,
}

impl Error {
    pub fn new(kind: Kind) -> Error {
        Error {
            kind: kind,
            reason: None,
        }
    }

    pub fn with(kind: Kind, reason: &str) -> Error {
        Error {
            kind: kind,
            reason: Some(String::from(reason)),
        }
    }
}

impl error::Error for Error {
    fn cause(&self) -> Option<&dyn error::Error> {
        None
    }

    fn description(&self) -> &str {
        match self.kind {
            Kind::Internal(_) => "internal error",
            Kind::UnreachableBackend => "unreachable backend",
            Kind::UnknownBackend => "unknown backend",
            Kind::UnsupportedVersion => "unsupported version",
            Kind::DuplicateVariable => "duplicate variable",
            Kind::DuplicateFunction => "duplicate function",
            Kind::IncompleteInput => "incomplete input",
            Kind::UnexpectedToken => "unexpected token",
            Kind::UnknownKeyword => "unknown keyword",
            Kind::StackExhausted => "stack exhausted",
            Kind::InvalidArgs => "invalid arguments",
            Kind::Io => "I/O failure",
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if let Some(ref reason) = self.reason {
            writeln!(f, "{}", reason).ok();
        }

        match self.kind {
            Kind::Internal(ref loc) => write!(f, "Internal error in {}", loc),
            Kind::UnreachableBackend => write!(f, "Unreachable backend"),
            Kind::UnknownBackend => write!(f, "Unknown backend"),
            Kind::UnsupportedVersion => write!(f, "Unsupported version"),
            Kind::DuplicateVariable => write!(f, "Duplicate variable"),
            Kind::DuplicateFunction => write!(f, "Duplicate function"),
            Kind::IncompleteInput => write!(f, "Incomplete input"),
            Kind::UnexpectedToken => write!(f, "Unexpected token"),
            Kind::UnknownKeyword => write!(f, "Unknown keyword"),
            Kind::StackExhausted => write!(f, "Stack exhausted"),
            Kind::InvalidArgs => write!(f, "Invalid arguments"),
            Kind::Io => write!(f, "I/O failure"),
        }
    }
}

#[macro_export]
macro_rules! error {
    ( $type:ident ) => {
        $crate::Error::new($crate::Kind::$type)
    };
    ( $type:ident, $message:expr ) => {
        $crate::Error::with($crate::Kind::$type, $message)
    };
}

#[macro_export]
macro_rules! exception {
    () => {
        $crate::Error::new($crate::Kind::Internal($crate::Location {
            filename: file!(),
            line: line!(),
            column: column!(),
        }))
    };
    ( $message:expr ) => {
        $crate::Error::with(
            $crate::Kind::Internal($crate::Location {
                filename: file!(),
                line: line!(),
                column: column!(),
            }),
            $message,
        );
    };
}

impl From<io::Error> for Error {
    fn from(err: io::Error) -> Error {
        let mut msg = String::new();
        write!(&mut msg, "{}", err).ok();
        error!(Io, &msg)
    }
}
