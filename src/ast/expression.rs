use crate::position::Position;

#[derive(Debug, Eq, PartialEq)]
pub enum Expression<'a> {
    List(&'a str, &'a str, Vec<Self>, Position),
    Quote(&'a str, Box<Self>, Position),
    QuotedSymbol(&'a str, Position),
    String(&'a str, Position),
    Symbol(&'a str, Position),
}

impl Expression<'_> {
    pub const fn position(&self) -> &Position {
        match self {
            Self::List(_, _, _, position) => position,
            Self::Quote(_, _, position) => position,
            Self::QuotedSymbol(_, position) => position,
            Self::String(_, position) => position,
            Self::Symbol(_, position) => position,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn equal() {
        assert_eq!(
            Expression::Symbol("foo", Position::new(0, 0)),
            Expression::Symbol("foo", Position::new(0, 0))
        );
    }
}
