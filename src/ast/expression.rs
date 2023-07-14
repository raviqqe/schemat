use crate::position::Position;
use std::alloc::Allocator;

#[derive(Debug)]
pub enum Expression<'a, A: Allocator> {
    List(Vec<Expression<'a, A>, A>, Position),
    Quote(Box<Expression<'a, A>, A>, Position),
    String(&'a str, Position),
    Symbol(&'a str, Position),
    Vector(Vec<Expression<'a, A>, A>, Position),
}

impl<'a, A: Allocator> Expression<'a, A> {
    pub fn position(&self) -> &Position {
        match self {
            Self::List(_, position) => position,
            Self::Quote(_, position) => position,
            Self::String(_, position) => position,
            Self::Symbol(_, position) => position,
            Self::Vector(_, position) => position,
        }
    }
}

// TODO Why do we need to do this manually?
impl<'a, A: Allocator> PartialEq for Expression<'a, A> {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::List(one, position), Self::List(other, other_position)) => {
                one == other && position == other_position
            }
            (Self::Quote(one, position), Self::Quote(other, other_position)) => {
                one == other && position == other_position
            }
            (Self::String(one, position), Self::String(other, other_position)) => {
                one == other && position == other_position
            }
            (Self::Symbol(one, position), Self::Symbol(other, other_position)) => {
                one == other && position == other_position
            }
            (Self::Vector(one, position), Self::Vector(other, other_position)) => {
                one == other && position == other_position
            }
            _ => false,
        }
    }
}

impl<'a, A: Allocator> Eq for Expression<'a, A> {}

#[cfg(test)]
mod tests {
    use super::*;
    use std::alloc::Global;

    #[test]
    fn equal() {
        assert_eq!(
            Expression::<Global>::Symbol("foo", Position::new(0, 0)),
            Expression::<Global>::Symbol("foo", Position::new(0, 0))
        );
    }
}
