use core::{
    error, fmt,
    fmt::{Display, Formatter},
    str::Utf8Error,
};
use glob::{GlobError, PatternError};
use std::io;

/// An error.
#[derive(Debug)]
pub enum Error {
    /// A format error.
    Format(fmt::Error),
    /// A Git error.
    Gix(gix::Error),
    /// A glob error.
    Glob(GlobError),
    /// An I/O error.
    Io(io::Error),
    /// A parse error.
    Parse(String),
    /// A glob pattern error.
    Pattern(PatternError),
    /// A UTF-8 error.
    Utf8(Utf8Error),
}

impl error::Error for Error {}

impl Display for Error {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Format(error) => error.fmt(formatter),
            Self::Gix(error) => error.fmt(formatter),
            Self::Glob(error) => error.fmt(formatter),
            Self::Io(error) => error.fmt(formatter),
            Self::Parse(error) => error.fmt(formatter),
            Self::Pattern(error) => error.fmt(formatter),
            Self::Utf8(error) => error.fmt(formatter),
        }
    }
}

impl From<fmt::Error> for Error {
    fn from(error: fmt::Error) -> Self {
        Self::Format(error)
    }
}

impl From<gix::Error> for Error {
    fn from(error: gix::Error) -> Self {
        Self::Gix(error)
    }
}

impl From<GlobError> for Error {
    fn from(error: GlobError) -> Self {
        Self::Glob(error)
    }
}

impl From<io::Error> for Error {
    fn from(error: io::Error) -> Self {
        Self::Io(error)
    }
}

impl From<PatternError> for Error {
    fn from(error: PatternError) -> Self {
        Self::Pattern(error)
    }
}

impl From<Utf8Error> for Error {
    fn from(error: Utf8Error) -> Self {
        Self::Utf8(error)
    }
}
