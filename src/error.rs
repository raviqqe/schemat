use core::{
    error::Error,
    fmt,
    fmt::{Display, Formatter},
};
use glob::{GlobError, PatternError};
use std::{io, str::Utf8Error};

#[derive(Debug)]
pub enum ApplicationError {
    Format(fmt::Error),
    GixOpenIndex(gix::worktree::open_index::Error),
    Glob(GlobError),
    Ignore(ignore::Error),
    Io(io::Error),
    Parse(String),
    Pattern(PatternError),
    Utf8(Utf8Error),
}

impl Error for ApplicationError {}

impl Display for ApplicationError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Format(error) => error.fmt(formatter),
            Self::GixOpenIndex(error) => error.fmt(formatter),
            Self::Glob(error) => error.fmt(formatter),
            Self::Ignore(error) => error.fmt(formatter),
            Self::Io(error) => error.fmt(formatter),
            Self::Parse(error) => error.fmt(formatter),
            Self::Pattern(error) => error.fmt(formatter),
            Self::Utf8(error) => error.fmt(formatter),
        }
    }
}

impl From<fmt::Error> for ApplicationError {
    fn from(error: fmt::Error) -> Self {
        Self::Format(error)
    }
}

impl From<gix::worktree::open_index::Error> for ApplicationError {
    fn from(error: gix::worktree::open_index::Error) -> Self {
        Self::GixOpenIndex(error)
    }
}

impl From<GlobError> for ApplicationError {
    fn from(error: GlobError) -> Self {
        Self::Glob(error)
    }
}

impl From<ignore::Error> for ApplicationError {
    fn from(error: ignore::Error) -> Self {
        Self::Ignore(error)
    }
}

impl From<io::Error> for ApplicationError {
    fn from(error: io::Error) -> Self {
        Self::Io(error)
    }
}

impl From<PatternError> for ApplicationError {
    fn from(error: PatternError) -> Self {
        Self::Pattern(error)
    }
}

impl From<Utf8Error> for ApplicationError {
    fn from(error: Utf8Error) -> Self {
        Self::Utf8(error)
    }
}
