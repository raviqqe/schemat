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
    Patttern(PatternError),
}

impl Error for ApplicationError {}

impl Display for ApplicationError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Glob(error) => error.fmt(f),
            Self::Io(error) => error.fmt(f),
            Self::Patttern(error) => error.fmt(f),
        }
    }
}

impl From<GlobError> for ApplicationError {
    fn from(error: GlobError) -> Self {
        Self::Glob(error)
    }
}

impl From<PatternError> for ApplicationError {
    fn from(error: PatternError) -> Self {
        Self::Patttern(error)
    }
}
