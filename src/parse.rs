mod error;
mod input;
mod parser;

pub use self::error::ParseError;
use self::{
    input::Input,
    parser::{IResult, comments, hash_directives, module},
};
use crate::ast::{Comment, Expression, HashDirective};

pub fn parse(source: &str) -> Result<Vec<Expression<'_>>, ParseError> {
    convert_result(module(Input::new(source)), source)
}

pub fn parse_comments(source: &str) -> Result<Vec<Comment<'_>>, ParseError> {
    convert_result(comments(Input::new(source)), source)
}

pub fn parse_hash_directives(source: &str) -> Result<Vec<HashDirective<'_>>, ParseError> {
    convert_result(hash_directives(Input::new(source)), source)
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
    use pretty_assertions::assert_eq;

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
            Ok(vec![Expression::Quote(
                "#",
                Expression::Symbol("t", Position::new(11, 12)).into(),
                Position::new(10, 12)
            )])
        );
    }

    #[test]
    fn parse_lang_directive() {
        assert_eq!(
            parse("#lang racket\n#t"),
            Ok(vec![Expression::Quote(
                "#",
                Expression::Symbol("t", Position::new(14, 15)).into(),
                Position::new(13, 15)
            )])
        );
    }

    #[test]
    fn parse_empty_list() {
        assert_eq!(
            parse("()"),
            Ok(vec![Expression::List(
                "(",
                ")",
                vec![],
                Position::new(0, 2)
            )])
        );
    }

    #[test]
    fn parse_list_with_element() {
        assert_eq!(
            parse("(foo)"),
            Ok(vec![Expression::List(
                "(",
                ")",
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
                "(",
                ")",
                vec![
                    Expression::Symbol("foo", Position::new(1, 4)),
                    Expression::Symbol("bar", Position::new(5, 8))
                ],
                Position::new(0, 9)
            )])
        );
    }

    #[test]
    fn parse_vector() {
        assert_eq!(
            parse("#()"),
            Ok(vec![Expression::Quote(
                "#",
                Expression::List("(", ")", vec![], Position::new(1, 3)).into(),
                Position::new(0, 3)
            )])
        );
    }

    #[test]
    fn parse_symbol_starting_with_escaped_hash() {
        assert_eq!(
            parse("\\#foo"),
            Ok(vec![Expression::Symbol("\\#foo", Position::new(0, 5)),])
        );
    }

    #[test]
    fn parse_symbol_quoted_by_hash_and_single_quote() {
        assert_eq!(
            parse("#'foo"),
            Ok(vec![Expression::Quote(
                "#",
                Expression::Quote(
                    "'",
                    Expression::Symbol("foo", Position::new(2, 5)).into(),
                    Position::new(1, 5)
                )
                .into(),
                Position::new(0, 5)
            )])
        );
    }

    #[test]
    fn parse_last_boolean_in_list() {
        assert_eq!(
            parse("(#f)"),
            Ok(vec![Expression::List(
                "(",
                ")",
                vec![Expression::Quote(
                    "#",
                    Expression::Symbol("f", Position::new(2, 3)).into(),
                    Position::new(1, 3)
                )],
                Position::new(0, 4)
            )])
        );
    }

    mod comment {
        use super::*;
        use crate::ast::{BlockComment, LineComment};
        use pretty_assertions::assert_eq;

        #[test]
        fn parse_block_comment() {
            assert_eq!(
                parse_comments("#|foo|#"),
                Ok(vec![BlockComment::new("foo", Position::new(0, 7)).into()])
            );
        }

        #[test]
        fn parse_line_comment() {
            assert_eq!(
                parse_comments(";foo\n"),
                Ok(vec![LineComment::new("foo", Position::new(0, 4)).into()])
            );
        }
    }

    mod hash {
        use super::*;
        use pretty_assertions::assert_eq;

        #[test]
        fn parse_directive() {
            assert_eq!(
                parse_hash_directives("#foo\n"),
                Ok(vec![HashDirective::new("foo", Position::new(0, 4))])
            );
        }

        #[test]
        fn parse_directives() {
            assert_eq!(
                parse_hash_directives("#foo\n#bar\n"),
                Ok(vec![
                    HashDirective::new("foo", Position::new(0, 4)),
                    HashDirective::new("bar", Position::new(5, 9))
                ])
            );
        }
    }
}
