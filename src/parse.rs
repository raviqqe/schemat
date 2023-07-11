mod error;
mod input;
mod parser;

use self::{error::NomError, input::Input, parser::module};
use crate::ast::Expression;

pub type ParseError<'a> = nom::Err<NomError<'a>>;

pub fn parse(source: &str) -> Result<Vec<Expression>, ParseError> {
    module(Input::new(source)).map(|(_, module)| module)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::position::Position;

    #[test]
    fn parse_nothing() {
        assert_eq!(parse(""), Ok(vec![]));
    }

    #[test]
    fn parse_symbol() {
        assert_eq!(
            parse("foo"),
            Ok(vec![Expression::Symbol("foo", Position::new(0, 3))])
        );
    }

    #[test]
    fn parse_empty_list() {
        assert_eq!(
            parse("()"),
            Ok(vec![Expression::List(vec![], Position::new(0, 2))])
        );
    }

    #[test]
    fn parse_list_with_element() {
        assert_eq!(
            parse("(foo)"),
            Ok(vec![Expression::List(
                vec![Expression::Symbol("foo", Position::new(1, 4))],
                Position::new(0, 5)
            )])
        );
    }

    #[test]
    fn parse_list_with_elements() {
        assert_eq!(
            parse("(foo bar)"),
            Ok(vec![Expression::List(
                vec![
                    Expression::Symbol("foo", Position::new(1, 4)),
                    Expression::Symbol("bar", Position::new(5, 8))
                ],
                Position::new(0, 9)
            )])
        );
    }
}
