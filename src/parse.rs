mod error;
mod input;
mod parser;

use self::{
    error::ParseError,
    input::Input,
    parser::{comments, hash_lines, module, IResult},
};
use crate::ast::{Comment, Expression, HashLine};

pub fn parse(source: &str) -> Result<Vec<Expression>, ParseError> {
    convert_result(module(Input::new(source)), source)
}

pub fn parse_comments(source: &str) -> Result<Vec<Comment>, ParseError> {
    convert_result(comments(Input::new(source)), source)
}

pub fn parse_hash_lines(source: &str) -> Result<Vec<HashLine>, ParseError> {
    convert_result(hash_lines(Input::new(source)), source)
}

fn convert_result<T>(result: IResult<T>, source: &str) -> Result<T, ParseError> {
    result
        .map(|(_, value)| value)
        .map_err(|error| ParseError::new(source, error))
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
    fn parse_shebang() {
        assert_eq!(
            parse("#!/bin/sh\n#t"),
            Ok(vec![Expression::Symbol("#t", Position::new(10, 12))])
        );
    }

    #[test]
    fn parse_lang_directive() {
        assert_eq!(
            parse("#lang racket\n#t"),
            Ok(vec![Expression::Symbol("#t", Position::new(13, 15))])
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
