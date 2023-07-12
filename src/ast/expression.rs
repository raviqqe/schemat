use crate::position::Position;

#[derive(Debug, Eq, PartialEq)]
pub enum Expression<'a> {
    List(Vec<Expression<'a>>, Position),
    Quote(Box<Expression<'a>>, Position),
    String(&'a str, Position),
    Symbol(&'a str, Position),
}

impl<'a> Expression<'a> {
    pub fn position(&self) -> &Position {
        match self {
            Self::List(_, position) => position,
            Self::Quote(_, position) => position,
            Self::String(_, position) => position,
            Self::Symbol(_, position) => position,
        }
    }
}
