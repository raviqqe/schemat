use crate::position::Position;
use std::alloc::Allocator;

#[derive(Debug, Eq, PartialEq)]
pub enum Expression<'a, A: Allocator> {
    List(Vec<Expression<'a, A>, A>, Position),
    Quote(Box<Expression<'a, A>>, Position),
    String(&'a str, Position),
    Symbol(&'a str, Position),
}

impl<'a, A: Allocator> Expression<'a, A> {
    pub fn position(&self) -> &Position {
        match self {
            Self::List(_, position) => position,
            Self::Quote(_, position) => position,
            Self::String(_, position) => position,
            Self::Symbol(_, position) => position,
        }
    }
}
