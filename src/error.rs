use core::fmt;
use glob::{GlobError, PatternError};
use std::{
    error::Error,
    fmt::{Display, Formatter},
};
use tokio::task::JoinError;

#[derive(Debug)]
pub enum ApplicationError {
    Glob(GlobError),
    Join(JoinError),
    Pattern(PatternError),
}

impl Error for ApplicationError {}

impl Display for ApplicationError {
    fn fmt(&self, formatter: &mut Formatter) -> fmt::Result {
        match self {
            Self::Glob(error) => error.fmt(formatter),
            Self::Join(error) => error.fmt(formatter),
            Self::Pattern(error) => error.fmt(formatter),
        }
    }
}

impl From<GlobError> for ApplicationError {
    fn from(error: GlobError) -> Self {
        Self::Glob(error)
    }
}

impl From<JoinError> for ApplicationError {
    fn from(error: JoinError) -> Self {
        Self::Join(error)
    }
}

impl From<PatternError> for ApplicationError {
    fn from(error: PatternError) -> Self {
        Self::Pattern(error)
    }
}
