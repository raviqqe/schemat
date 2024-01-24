use core::{
    error::Error,
    fmt,
    fmt::{Display, Formatter},
};
use glob::{GlobError, PatternError};
use std::io;

#[derive(Debug)]
pub enum ApplicationError {
    Glob(GlobError),
    Io(io::Error),
    Parse(String),
    Patttern(PatternError),
}

impl Error for ApplicationError {}

impl Display for ApplicationError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Glob(error) => error.fmt(formatter),
            Self::Io(error) => error.fmt(formatter),
            Self::Parse(error) => error.fmt(formatter),
            Self::Patttern(error) => error.fmt(formatter),
        }
    }
}

impl From<GlobError> for ApplicationError {
    fn from(error: GlobError) -> Self {
        Self::Glob(error)
    }
}

impl From<io::Error> for ApplicationError {
    fn from(error: io::Error) -> Self {
        Self::Io(error)
    }
}

impl From<PatternError> for ApplicationError {
    fn from(error: PatternError) -> Self {
        Self::Patttern(error)
    }
}
