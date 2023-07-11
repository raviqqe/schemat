mod input;
mod parser;

use self::{
    input::Input,
    parser::{module, Error},
};
use crate::ast::Expression;

pub type ParseError<'a> = nom::Err<Error<'a>>;

pub fn parse(source: &str) -> Result<Vec<Expression>, ParseError> {
    Ok(module(Input::new(source)).map(|(_, module)| module)?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_nothing() {
        assert_eq!(parse(""), Ok(vec![]));
    }

    #[test]
    fn parse_symbol() {
        assert_eq!(parse("foo"), Ok(vec![Expression::Symbol("foo")]));
    }

    #[test]
    fn parse_empty_list() {
        assert_eq!(parse("()"), Ok(vec![Expression::List(vec![])]));
    }

    #[test]
    fn parse_list_with_element() {
        assert_eq!(
            parse("(foo)"),
            Ok(vec![Expression::List(vec![Expression::Symbol("foo")])])
        );
    }

    #[test]
    fn parse_list_with_elements() {
        assert_eq!(
            parse("(foo bar)"),
            Ok(vec![Expression::List(vec![
                Expression::Symbol("foo"),
                Expression::Symbol("bar")
            ])])
        );
    }
}
