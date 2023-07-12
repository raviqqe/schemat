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

#[derive(Debug, Eq, PartialEq)]
pub struct Comment<'a> {
    value: &'a str,
    position: Position,
}

impl<'a> Comment<'a> {
    pub fn new(value: &'a str, position: Position) -> Self {
        Self { value, position }
    }

    pub fn value(&self) -> &'a str {
        self.value
    }

    pub fn position(&self) -> &Position {
        &self.position
    }
}
