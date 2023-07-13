mod error;
mod input;
mod parser;

pub use self::error::ParseError;
use self::{
    input::Input,
    parser::{comments, hash_directives, module, IResult},
};
use crate::ast::{Comment, Expression, HashDirective};
use std::alloc::Allocator;

pub fn parse<A: Allocator + Clone>(
    source: &str,
    allocator: A,
) -> Result<Vec<Expression<A>, A>, ParseError> {
    convert_result(module(Input::new_extra(source, allocator)), source)
}

pub fn parse_comments<A: Allocator + Clone>(
    source: &str,
    allocator: A,
) -> Result<Vec<Comment, A>, ParseError> {
    convert_result(comments(Input::new_extra(source, allocator)), source)
}

pub fn parse_hash_directives<A: Allocator + Clone>(
    source: &str,
    allocator: A,
) -> Result<Vec<HashDirective, A>, ParseError> {
    convert_result(hash_directives(Input::new_extra(source, allocator)), source)
}

fn convert_result<T, A: Allocator + Clone>(
    result: IResult<T, A>,
    source: &str,
) -> Result<T, ParseError> {
    result
        .map(|(_, value)| value)
        .map_err(|error| ParseError::new(source, error))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::position::Position;
    use std::alloc::Global;

    #[test]
    fn parse_nothing() {
        assert_eq!(parse("", Global), Ok(vec![]));
    }

    #[test]
    fn parse_symbol() {
        assert_eq!(
            parse("foo", Global),
            Ok(vec![Expression::Symbol("foo", Position::new(0, 3))])
        );
    }

    #[test]
    fn parse_shebang() {
        assert_eq!(
            parse("#!/bin/sh\n#t", Global),
            Ok(vec![Expression::Symbol("#t", Position::new(10, 12))])
        );
    }

    #[test]
    fn parse_lang_directive() {
        assert_eq!(
            parse("#lang racket\n#t", Global),
            Ok(vec![Expression::Symbol("#t", Position::new(13, 15))])
        );
    }

    #[test]
    fn parse_empty_list() {
        assert_eq!(
            parse("()", Global),
            Ok(vec![Expression::List(vec![], Position::new(0, 2))])
        );
    }

    #[test]
    fn parse_list_with_element() {
        assert_eq!(
            parse("(foo)", Global),
            Ok(vec![Expression::List(
                vec![Expression::Symbol("foo", Position::new(1, 4))],
                Position::new(0, 5)
            )])
        );
    }

    #[test]
    fn parse_list_with_elements() {
        assert_eq!(
            parse("(foo bar)", Global),
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
