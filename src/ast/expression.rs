use crate::position::Position;
use std::alloc::Allocator;

#[derive(Debug)]
pub enum Expression<'a, A: Allocator> {
    List(&'a str, &'a str, Vec<Expression<'a, A>, A>, Position),
    Quote(&'a str, Box<Expression<'a, A>, A>, Position),
    QuotedSymbol(&'a str, Position),
    String(&'a str, Position),
    Symbol(&'a str, Position),
}

impl<A: Allocator> Expression<'_, A> {
    pub fn position(&self) -> &Position {
        match self {
            Self::List(_, _, _, position) => position,
            Self::Quote(_, _, position) => position,
            Self::QuotedSymbol(_, position) => position,
            Self::String(_, position) => position,
            Self::Symbol(_, position) => position,
        }
    }
}

// TODO Why do we need to do this manually?
impl<A: Allocator> PartialEq for Expression<'_, A> {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (
                Self::List(left, right, one, position),
                Self::List(other_left, other_right, other, other_position),
            ) => {
                left == other_left
                    && right == other_right
                    && one == other
                    && position == other_position
            }
            (Self::Quote(sign, one, position), Self::Quote(other_sign, other, other_position)) => {
                sign == other_sign && one == other && position == other_position
            }
            (Self::QuotedSymbol(one, position), Self::QuotedSymbol(other, other_position)) => {
                one == other && position == other_position
            }
            (Self::String(one, position), Self::String(other, other_position)) => {
                one == other && position == other_position
            }
            (Self::Symbol(one, position), Self::Symbol(other, other_position)) => {
                one == other && position == other_position
            }
            _ => false,
        }
    }
}

impl<A: Allocator> Eq for Expression<'_, A> {}

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
