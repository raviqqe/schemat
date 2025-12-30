use core::{
    error::Error,
    fmt,
    fmt::{Display, Formatter},
};
use std::io;

#[derive(Debug)]
pub enum ApplicationError {
    Format(fmt::Error),
    Glob(ignore::Error),
    Io(io::Error),
    Parse(String),
}

impl Error for ApplicationError {}

impl Display for ApplicationError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Format(error) => error.fmt(formatter),
            Self::Glob(error) => error.fmt(formatter),
            Self::Io(error) => error.fmt(formatter),
            Self::Parse(error) => error.fmt(formatter),
        }
    }
}

impl From<fmt::Error> for ApplicationError {
    fn from(error: fmt::Error) -> Self {
        Self::Format(error)
    }
}

impl From<ignore::Error> for ApplicationError {
    fn from(error: ignore::Error) -> Self {
        Self::Glob(error)
    }
}

impl From<io::Error> for ApplicationError {
    fn from(error: io::Error) -> Self {
        Self::Io(error)
    }
}
