use super::{error::NomError, input::Input};
use crate::{
    ast::{BlockComment, Comment, Expression, HashDirective, LineComment},
    position::Position,
};
use nom::{
    branch::alt,
    bytes::complete::{tag, take_until},
    character::complete::{anychar, char, multispace0, multispace1, none_of, satisfy, space0},
    combinator::{all_consuming, cut, eof, map, not, peek, recognize, value},
    error::context,
    multi::{fold_many0, many0_count, many1, many1_count},
    sequence::{delimited, preceded, terminated, tuple},
    Parser,
};
use std::alloc::Allocator;

const SYMBOL_SIGNS: &str = "+-*/<>=!?$@%_&~^.:";

pub type IResult<'a, T, A> = nom::IResult<Input<'a, A>, T, NomError<'a, A>>;

pub fn module<A: Allocator + Clone>(input: Input<A>) -> IResult<Vec<Expression<A>, A>, A> {
    all_consuming(delimited(
        many0_count(hash_directive),
        many0(expression),
        blank,
    ))(input)
}

pub fn comments<A: Allocator + Clone>(input: Input<A>) -> IResult<Vec<Comment, A>, A> {
    let allocator = input.extra.clone();

    all_consuming(fold_many0(
        alt((
            map(none_of("\";#\\"), |_| None),
            map(raw_string, |_| None),
            map(raw_symbol, |_| None),
            map(comment, Some),
            map(quote, |_| None),
        )),
        move || Vec::new_in(allocator.clone()),
        |mut all, comment| {
            if let Some(comment) = comment {
                all.push(comment);
            }

            all
        },
    ))(input)
}

pub fn hash_directives<A: Allocator + Clone>(input: Input<A>) -> IResult<Vec<HashDirective, A>, A> {
    many0(hash_directive)(input)
}

fn symbol<A: Allocator + Clone>(input: Input<A>) -> IResult<Expression<A>, A> {
    map(token(positioned(raw_symbol)), |(input, position)| {
        Expression::Symbol(&input, position)
    })(input)
}

fn raw_symbol<A: Allocator + Clone>(input: Input<A>) -> IResult<Input<A>, A> {
    recognize(tuple((head_symbol_character, many0(tail_symbol_character))))(input)
}

fn head_symbol_character<A: Allocator + Clone>(input: Input<A>) -> IResult<Input<A>, A> {
    recognize(alt((
        value(
            (),
            satisfy(|character| character.is_alphanumeric() || SYMBOL_SIGNS.contains(character)),
        ),
        value((), tuple((char('\\'), anychar))),
    )))(input)
}

fn tail_symbol_character<A: Allocator + Clone>(input: Input<A>) -> IResult<Input<A>, A> {
    alt((head_symbol_character, recognize(char('#'))))(input)
}

fn expression<A: Allocator + Clone>(input: Input<A>) -> IResult<Expression<A>, A> {
    let allocator = input.extra.clone();

    alt((
        context("list", list_like("(", ")")),
        context("string", string),
        context(
            "quote",
            map(
                token(positioned(tuple((quote, expression)))),
                move |((sign, expression), position)| {
                    Expression::Quote(&sign, Box::new_in(expression, allocator.clone()), position)
                },
            ),
        ),
        context("symbol", symbol),
        context("vector", list_like("[", "]")),
        context("map", list_like("{", "}")),
    ))(input)
}

fn quote<A: Allocator + Clone>(input: Input<A>) -> IResult<Input<A>, A> {
    alt((
        tag("'"),
        tag("`"),
        tag(",@"),
        tag(","),
        tag("#;"),
        tag("#"),
        terminated(raw_symbol, peek(not(alt((multispace1, eof))))),
    ))(input)
}

fn list_like<'a, A: Allocator + Clone>(
    left: &'static str,
    right: &'static str,
) -> impl FnMut(Input<'a, A>) -> IResult<Expression<'a, A>, A> {
    move |input| {
        map(
            token(positioned(tuple((
                sign(left),
                cut(tuple((many0(expression), sign(right)))),
            )))),
            |((left, (expressions, right)), position)| {
                Expression::List(&left, &right, expressions, position)
            },
        )(input)
    }
}

fn string<A: Allocator + Clone>(input: Input<A>) -> IResult<Expression<A>, A> {
    map(token(positioned(raw_string)), |(input, position)| {
        Expression::String(*input, position)
    })(input)
}

fn raw_string<A: Allocator + Clone>(input: Input<A>) -> IResult<Input<A>, A> {
    delimited(
        char('"'),
        recognize(many0(alt((
            recognize(none_of("\\\"")),
            tag("\\\\"),
            tag("\\\""),
            tag("\\'"),
            tag("\\n"),
            tag("\\r"),
            tag("\\t"),
            tag("\\\n"),
            recognize(tuple((char('\\'), hexadecimal_digit, hexadecimal_digit))),
            recognize(preceded(
                tag("\\x"),
                cut(terminated(
                    many1(tuple((hexadecimal_digit, hexadecimal_digit))),
                    char(';'),
                )),
            )),
        )))),
        char('"'),
    )(input)
}

fn hexadecimal_digit<A: Allocator + Clone>(input: Input<A>) -> IResult<char, A> {
    satisfy(|character| character.is_ascii_hexdigit())(input)
}

fn sign<A: Allocator + Clone>(sign: &'static str) -> impl Fn(Input<A>) -> IResult<Input<A>, A> {
    move |input| token(tag(sign))(input)
}

fn token<'a, T, A: Allocator + Clone>(
    mut parser: impl Parser<Input<'a, A>, T, NomError<'a, A>>,
) -> impl FnMut(Input<'a, A>) -> IResult<T, A> {
    move |input| preceded(blank, |input| parser.parse(input))(input)
}

fn positioned<'a, T, A: Allocator + Clone>(
    mut parser: impl Parser<Input<'a, A>, T, NomError<'a, A>>,
) -> impl FnMut(Input<'a, A>) -> IResult<'a, (T, Position), A> {
    move |input| {
        map(
            tuple((
                nom_locate::position,
                |input| parser.parse(input),
                nom_locate::position,
            )),
            |(start, value, end)| {
                (
                    value,
                    Position::new(start.location_offset(), end.location_offset()),
                )
            },
        )(input)
    }
}

fn positioned_meta<'a, T, A: Allocator + Clone>(
    mut parser: impl Parser<Input<'a, A>, T, NomError<'a, A>>,
) -> impl FnMut(Input<'a, A>) -> IResult<'a, (T, Position), A> {
    move |input| {
        map(
            tuple((
                preceded(multispace0, nom_locate::position),
                |input| parser.parse(input),
                nom_locate::position,
            )),
            |(start, value, end)| {
                (
                    value,
                    Position::new(start.location_offset(), end.location_offset()),
                )
            },
        )(input)
    }
}

fn blank<A: Allocator + Clone>(input: Input<A>) -> IResult<(), A> {
    value(
        (),
        many0_count(alt((value((), multispace1), value((), comment)))),
    )(input)
}

fn comment<A: Allocator + Clone>(input: Input<A>) -> IResult<Comment, A> {
    alt((
        map(line_comment, From::from),
        map(block_comment, From::from),
    ))(input)
}

fn line_comment<A: Allocator + Clone>(input: Input<A>) -> IResult<LineComment, A> {
    map(
        terminated(
            positioned_meta(preceded(char(';'), take_until("\n"))),
            newline,
        ),
        |(input, position)| LineComment::new(&input, position),
    )(input)
}

fn block_comment<A: Allocator + Clone>(input: Input<A>) -> IResult<BlockComment, A> {
    map(
        positioned_meta(delimited(
            tag("#|"),
            recognize(many0(tuple((not(tag("|#")), anychar)))),
            tag("|#"),
        )),
        |(input, position)| BlockComment::new(&input, position),
    )(input)
}

fn hash_directive<A: Allocator + Clone>(input: Input<A>) -> IResult<HashDirective, A> {
    map(
        terminated(
            positioned_meta(preceded(
                tuple((char('#'), not(peek(char('|'))))),
                take_until("\n"),
            )),
            newline,
        ),
        |(input, position)| HashDirective::new(&input, position),
    )(input)
}

fn newline<A: Allocator + Clone>(input: Input<A>) -> IResult<(), A> {
    value(
        (),
        many1_count(delimited(space0, nom::character::complete::newline, space0)),
    )(input)
}

fn many0<'a, T, A: Allocator + Clone>(
    mut parser: impl Parser<Input<'a, A>, T, NomError<'a, A>>,
) -> impl FnMut(Input<'a, A>) -> IResult<Vec<T, A>, A> {
    move |input| {
        let allocator = input.extra.clone();

        fold_many0(
            |input| parser.parse(input),
            move || Vec::new_in(allocator.clone()),
            |mut all, value| {
                all.push(value);
                all
            },
        )(input)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;
    use std::alloc::Global;

    #[test]
    fn parse_symbol() {
        assert_eq!(
            expression(Input::new_extra("x", Global)).unwrap().1,
            Expression::Symbol("x", Position::new(0, 1))
        );
        assert_eq!(
            expression(Input::new_extra("foo", Global)).unwrap().1,
            Expression::Symbol("foo", Position::new(0, 3))
        );
        assert_eq!(
            expression(Input::new_extra("1", Global)).unwrap().1,
            Expression::Symbol("1", Position::new(0, 1))
        );
        assert_eq!(
            expression(Input::new_extra("42", Global)).unwrap().1,
            Expression::Symbol("42", Position::new(0, 2))
        );
        assert_eq!(
            expression(Input::new_extra("3.14", Global)).unwrap().1,
            Expression::Symbol("3.14", Position::new(0, 4))
        );
        assert_eq!(
            expression(Input::new_extra("a#a", Global)).unwrap().1,
            Expression::Symbol("a#a", Position::new(0, 3))
        );
        assert_eq!(
            expression(Input::new_extra("\\#", Global)).unwrap().1,
            Expression::Symbol("\\#", Position::new(0, 2))
        );
    }

    #[test]
    fn parse_invalid_symbol() {
        assert!(expression(Input::new_extra("#", Global)).is_err());
    }

    #[test]
    fn parse_list() {
        assert_eq!(
            expression(Input::new_extra("(1 2 3)", Global)).unwrap().1,
            Expression::List(
                "(",
                ")",
                vec![
                    Expression::Symbol("1", Position::new(1, 2)),
                    Expression::Symbol("2", Position::new(3, 4)),
                    Expression::Symbol("3", Position::new(5, 6))
                ],
                Position::new(0, 7)
            )
        );
    }

    #[test]
    fn parse_list_with_correct_position() {
        assert_eq!(
            expression(Input::new_extra(" ()", Global)).unwrap().1,
            Expression::List("(", ")", vec![], Position::new(1, 3))
        );
    }

    #[test]
    fn parse_character() {
        assert_eq!(
            expression(Input::new_extra("#\\a", Global)).unwrap().1,
            Expression::Quote(
                "#",
                Expression::Symbol("\\a", Position::new(1, 3)).into(),
                Position::new(0, 3)
            )
        );
        assert_eq!(
            expression(Input::new_extra("#\\(", Global)).unwrap().1,
            Expression::Quote(
                "#",
                Expression::Symbol("\\(", Position::new(1, 3)).into(),
                Position::new(0, 3)
            )
        );
        assert_eq!(
            expression(Input::new_extra("#\\;", Global)).unwrap().1,
            Expression::Quote(
                "#",
                Expression::Symbol("\\;", Position::new(1, 3)).into(),
                Position::new(0, 3)
            )
        );
        assert_eq!(
            expression(Input::new_extra("#\\ ", Global)).unwrap().1,
            Expression::Quote(
                "#",
                Expression::Symbol("\\ ", Position::new(1, 3)).into(),
                Position::new(0, 3)
            )
        );
        assert_eq!(
            expression(Input::new_extra("#\\space", Global)).unwrap().1,
            Expression::Quote(
                "#",
                Expression::Symbol("\\space", Position::new(1, 7)).into(),
                Position::new(0, 7)
            )
        );
        assert_eq!(
            expression(Input::new_extra("#\\\n", Global)).unwrap().1,
            Expression::Quote(
                "#",
                Expression::Symbol("\\\n", Position::new(1, 3)).into(),
                Position::new(0, 3)
            )
        );
    }

    #[test]
    fn parse_vector() {
        assert_eq!(
            expression(Input::new_extra("#(1 2 3)", Global)).unwrap().1,
            Expression::Quote(
                "#",
                Expression::List(
                    "(",
                    ")",
                    vec![
                        Expression::Symbol("1", Position::new(2, 3)),
                        Expression::Symbol("2", Position::new(4, 5)),
                        Expression::Symbol("3", Position::new(6, 7))
                    ],
                    Position::new(1, 8)
                )
                .into(),
                Position::new(0, 8)
            )
        );
    }

    #[test]
    fn parse_byte_vector() {
        assert_eq!(
            expression(Input::new_extra("#u8(1 2 3)", Global))
                .unwrap()
                .1,
            Expression::Quote(
                "#",
                Expression::Quote(
                    "u8",
                    Expression::List(
                        "(",
                        ")",
                        vec![
                            Expression::Symbol("1", Position::new(4, 5)),
                            Expression::Symbol("2", Position::new(6, 7)),
                            Expression::Symbol("3", Position::new(8, 9))
                        ],
                        Position::new(3, 10)
                    )
                    .into(),
                    Position::new(1, 10)
                )
                .into(),
                Position::new(0, 10)
            ),
        );
    }

    #[test]
    fn parse_bracket_vector() {
        assert_eq!(
            expression(Input::new_extra("[1 2 3]", Global)).unwrap().1,
            Expression::List(
                "[",
                "]",
                vec![
                    Expression::Symbol("1", Position::new(1, 2)),
                    Expression::Symbol("2", Position::new(3, 4)),
                    Expression::Symbol("3", Position::new(5, 6))
                ],
                Position::new(0, 7)
            )
        );
    }

    #[test]
    fn parse_map() {
        assert_eq!(
            expression(Input::new_extra("#{1 2 3}", Global)).unwrap().1,
            Expression::Quote(
                "#",
                Expression::List(
                    "{",
                    "}",
                    vec![
                        Expression::Symbol("1", Position::new(2, 3)),
                        Expression::Symbol("2", Position::new(4, 5)),
                        Expression::Symbol("3", Position::new(6, 7))
                    ],
                    Position::new(1, 8)
                )
                .into(),
                Position::new(0, 8)
            )
        );
    }

    mod boolean {
        use super::*;
        use pretty_assertions::assert_eq;

        #[test]
        fn parse_false() {
            assert_eq!(
                expression(Input::new_extra("#f", Global)).unwrap().1,
                Expression::Quote(
                    "#",
                    Expression::Symbol("f", Position::new(1, 2)).into(),
                    Position::new(0, 2)
                )
            );
            assert_eq!(
                expression(Input::new_extra("#false", Global)).unwrap().1,
                Expression::Quote(
                    "#",
                    Expression::Symbol("false", Position::new(1, 6)).into(),
                    Position::new(0, 6)
                )
            );
        }

        #[test]
        fn parse_true() {
            assert_eq!(
                expression(Input::new_extra("#t", Global)).unwrap().1,
                Expression::Quote(
                    "#",
                    Expression::Symbol("t", Position::new(1, 2)).into(),
                    Position::new(0, 2)
                )
            );
            assert_eq!(
                expression(Input::new_extra("#true", Global)).unwrap().1,
                Expression::Quote(
                    "#",
                    Expression::Symbol("true", Position::new(1, 5)).into(),
                    Position::new(0, 5)
                )
            );
        }

        #[test]
        fn parse_boolean_followed_by_comment() {
            assert_eq!(
                expression(Input::new_extra("#f;", Global)).unwrap().1,
                Expression::Quote(
                    "#",
                    Expression::Symbol("f", Position::new(1, 2)).into(),
                    Position::new(0, 2)
                )
            );
        }

        #[test]
        fn parse_boolean_followed_by_right_parenthesis() {
            assert_eq!(
                expression(Input::new_extra("#f)", Global)).unwrap().1,
                Expression::Quote(
                    "#",
                    Expression::Symbol("f", Position::new(1, 2)).into(),
                    Position::new(0, 2)
                )
            );
        }
    }

    mod quote {
        use super::*;
        use pretty_assertions::assert_eq;

        #[test]
        fn parse_quote() {
            assert_eq!(
                expression(Input::new_extra("'foo", Global)).unwrap().1,
                Expression::Quote(
                    "'",
                    Expression::Symbol("foo", Position::new(1, 4)).into(),
                    Position::new(0, 4)
                )
            );
        }

        #[test]
        fn parse_quote_with_correct_position() {
            assert_eq!(
                expression(Input::new_extra(" 'foo", Global)).unwrap().1,
                Expression::Quote(
                    "'",
                    Expression::Symbol("foo", Position::new(2, 5)).into(),
                    Position::new(1, 5)
                )
            );
        }

        #[test]
        fn parse_unquote() {
            assert_eq!(
                expression(Input::new_extra(",foo", Global)).unwrap().1,
                Expression::Quote(
                    ",",
                    Expression::Symbol("foo", Position::new(1, 4)).into(),
                    Position::new(0, 4)
                )
            );
        }

        #[test]
        fn parse_hash_quote() {
            assert_eq!(
                expression(Input::new_extra("#()", Global)).unwrap().1,
                Expression::Quote(
                    "#",
                    Expression::List("(", ")", vec![], Position::new(1, 3)).into(),
                    Position::new(0, 3)
                )
            );
        }

        #[test]
        fn parse_hash_semicolon_quote() {
            assert_eq!(
                expression(Input::new_extra("#;()", Global)).unwrap().1,
                Expression::Quote(
                    "#;",
                    Expression::List("(", ")", vec![], Position::new(2, 4)).into(),
                    Position::new(0, 4)
                )
            );
        }

        #[test]
        fn parse_quasi_quote() {
            assert_eq!(
                expression(Input::new_extra("`foo", Global)).unwrap().1,
                Expression::Quote(
                    "`",
                    Expression::Symbol("foo", Position::new(1, 4)).into(),
                    Position::new(0, 4)
                )
            );
        }

        #[test]
        fn parse_splicing_unquote() {
            assert_eq!(
                expression(Input::new_extra(",@foo", Global)).unwrap().1,
                Expression::Quote(
                    ",@",
                    Expression::Symbol("foo", Position::new(2, 5)).into(),
                    Position::new(0, 5)
                )
            );
        }

        #[test]
        fn parse_symbol_and_quoted_list() {
            assert_eq!(
                tuple((expression, expression))(Input::new_extra("#u8 ()", Global))
                    .unwrap()
                    .1,
                (
                    Expression::Quote(
                        "#",
                        Expression::Symbol("u8", Position::new(1, 3)).into(),
                        Position::new(0, 3)
                    ),
                    Expression::List("(", ")", vec![], Position::new(4, 6))
                )
            );
        }

        #[test]
        fn parse_quote_with_space() {
            assert_eq!(
                expression(Input::new_extra("' foo", Global)).unwrap().1,
                Expression::Quote(
                    "'",
                    Expression::Symbol("foo", Position::new(2, 5)).into(),
                    Position::new(0, 5)
                )
            );
        }
    }

    mod hash_directive {
        use super::*;
        use pretty_assertions::assert_eq;

        #[test]
        fn parse_shebang() {
            assert_eq!(
                hash_directive(Input::new_extra("#!/bin/sh\n", Global))
                    .unwrap()
                    .1,
                HashDirective::new("!/bin/sh", Position::new(0, 9))
            );
        }

        #[test]
        fn parse_lang_directive() {
            assert_eq!(
                hash_directive(Input::new_extra("#lang r7rs\n", Global))
                    .unwrap()
                    .1,
                HashDirective::new("lang r7rs", Position::new(0, 10))
            );
        }

        #[test]
        fn parse_comment() {
            assert_eq!(
                hash_directives(Input::new_extra("#||#\n", Global))
                    .unwrap()
                    .1,
                vec![]
            );
        }
    }

    mod string {
        use super::*;
        use pretty_assertions::assert_eq;

        #[test]
        fn parse_empty() {
            assert_eq!(
                string(Input::new_extra("\"\"", Global)).unwrap().1,
                Expression::String("", Position::new(0, 2))
            );
        }

        #[test]
        fn parse_non_empty() {
            assert_eq!(
                string(Input::new_extra("\"foo\"", Global)).unwrap().1,
                Expression::String("foo", Position::new(0, 5))
            );
        }

        #[test]
        fn parse_escaped_double_quote() {
            assert_eq!(
                string(Input::new_extra("\"\\\"\"", Global)).unwrap().1,
                Expression::String("\\\"", Position::new(0, 4))
            );
        }

        #[test]
        fn parse_escaped_single_quote() {
            assert_eq!(
                string(Input::new_extra("\"\\'\"", Global)).unwrap().1,
                Expression::String("\\'", Position::new(0, 4))
            );
        }

        #[test]
        fn parse_escaped_characters() {
            assert_eq!(
                string(Input::new_extra("\"\\\\\\n\\r\\t\"", Global))
                    .unwrap()
                    .1,
                Expression::String("\\\\\\n\\r\\t", Position::new(0, 10))
            );
        }

        #[test]
        fn parse_scheme_hexadecimal_bytes() {
            assert_eq!(
                string(Input::new_extra("\"\\x0F;\"", Global)).unwrap().1,
                Expression::String("\\x0F;", Position::new(0, 7))
            );
            assert_eq!(
                string(Input::new_extra("\"\\xABCD;\"", Global)).unwrap().1,
                Expression::String("\\xABCD;", Position::new(0, 9))
            );
        }

        // https://webassembly.github.io/spec/core/text/values.html#strings
        #[test]
        fn parse_wasm_hexadecimal_bytes() {
            assert_eq!(
                string(Input::new_extra("\"\\00\\FF\"", Global)).unwrap().1,
                Expression::String("\\00\\FF", Position::new(0, 8))
            );
        }

        #[test]
        fn parse_multi_line() {
            assert_eq!(
                string(Input::new_extra("\"a\\\nb\"", Global)).unwrap().1,
                Expression::String("a\\\nb", Position::new(0, 6))
            );
        }
    }

    mod comment {
        use super::*;
        use pretty_assertions::assert_eq;

        #[test]
        fn parse_empty() {
            assert_eq!(
                comment(Input::new_extra(";\n", Global)).unwrap().1,
                LineComment::new("", Position::new(0, 1)).into()
            );
        }

        #[test]
        fn parse_comment() {
            assert_eq!(
                comment(Input::new_extra(";foo\n", Global)).unwrap().1,
                LineComment::new("foo", Position::new(0, 4)).into()
            );
        }

        #[test]
        fn parse_comments() {
            assert_eq!(
                comments(Input::new_extra(";foo\n;bar\n", Global))
                    .unwrap()
                    .1,
                vec![
                    LineComment::new("foo", Position::new(0, 4)).into(),
                    LineComment::new("bar", Position::new(5, 9)).into()
                ]
            );
        }

        #[test]
        fn parse_comments_with_blank_lines() {
            assert_eq!(
                comments(Input::new_extra(";foo\n\n;bar\n", Global))
                    .unwrap()
                    .1,
                vec![
                    LineComment::new("foo", Position::new(0, 4)).into(),
                    LineComment::new("bar", Position::new(6, 10)).into()
                ]
            );
        }

        #[test]
        fn parse_comments_skipping_hash_semicolon() {
            assert_eq!(
                comments(Input::new_extra("#;foo\n;bar\n", Global))
                    .unwrap()
                    .1,
                vec![LineComment::new("bar", Position::new(6, 10)).into()]
            );
        }

        #[test]
        fn parse_comments_skipping_hash_character() {
            assert_eq!(
                comments(Input::new_extra("#foo\n;bar\n", Global))
                    .unwrap()
                    .1,
                vec![LineComment::new("bar", Position::new(5, 9)).into()]
            );
        }

        #[test]
        fn parse_comment_character() {
            assert_eq!(
                comments(Input::new_extra("#\\;foo\n", Global)).unwrap().1,
                vec![]
            );
        }

        #[test]
        fn parse_comment_in_list() {
            assert_eq!(
                comments(Input::new_extra("(f\n;foo\nx)", Global))
                    .unwrap()
                    .1,
                vec![LineComment::new("foo", Position::new(3, 7)).into()]
            );
        }

        #[test]
        fn parse_comment_with_vector() {
            assert_eq!(comments(Input::new_extra("#()", Global)).unwrap().1, vec![]);
        }

        mod block {
            use super::*;
            use pretty_assertions::assert_eq;

            #[test]
            fn parse_empty() {
                assert_eq!(
                    block_comment(Input::new_extra("#||#", Global)).unwrap().1,
                    BlockComment::new("", Position::new(0, 4))
                );
            }

            #[test]
            fn parse_one_line() {
                assert_eq!(
                    block_comment(Input::new_extra("#|foo|#", Global))
                        .unwrap()
                        .1,
                    BlockComment::new("foo", Position::new(0, 7))
                );
            }

            #[test]
            fn parse_multi_line() {
                assert_eq!(
                    // spell-checker: disable-next-line
                    block_comment(Input::new_extra("#|\nfoo\nbar\nbaz\n|#", Global))
                        .unwrap()
                        .1,
                    // spell-checker: disable-next-line
                    BlockComment::new("\nfoo\nbar\nbaz\n", Position::new(0, 17))
                );
            }

            #[test]
            fn parse_in_comments() {
                assert_eq!(
                    comments(Input::new_extra("#|foo|#", Global)).unwrap().1,
                    vec![BlockComment::new("foo", Position::new(0, 7)).into()]
                );
            }
        }
    }
}
