use crate::position::Position;
use std::alloc::Allocator;

#[derive(Debug)]
pub enum Expression<'a, A: Allocator> {
    List(&'a str, Vec<Expression<'a, A>, A>, &'a str, Position),
    Quote(&'a str, Box<Expression<'a, A>, A>, Position),
    String(&'a str, Position),
    Symbol(&'a str, Position),
}

impl<'a, A: Allocator> Expression<'a, A> {
    pub fn position(&self) -> &Position {
        match self {
            Self::List(_, _, _, position) => position,
            Self::Quote(_, _, position) => position,
            Self::String(_, position) => position,
            Self::Symbol(_, position) => position,
        }
    }
}

// TODO Why do we need to do this manually?
impl<'a, A: Allocator> PartialEq for Expression<'a, A> {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (
                Self::List(left, one, right, position),
                Self::List(other_left, other, other_right, other_position),
            ) => {
                left == other_left
                    && right == other_right
                    && one == other
                    && position == other_position
            }
            (Self::Quote(sign, one, position), Self::Quote(other_sign, other, other_position)) => {
                sign == other_sign && one == other && position == other_position
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
