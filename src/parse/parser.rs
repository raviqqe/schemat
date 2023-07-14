use super::{error::NomError, input::Input};
use crate::{
    ast::{Comment, Expression, HashDirective},
    position::Position,
};
use nom::{
    branch::alt,
    bytes::complete::{tag, take_until, take_while1},
    character::complete::{char, multispace0, multispace1, none_of, one_of, space0},
    combinator::{all_consuming, cut, map, recognize, value},
    error::context,
    multi::{fold_many0, many0_count, many1_count},
    sequence::{delimited, preceded, terminated, tuple},
    Parser,
};
use std::alloc::Allocator;

const SYMBOL_SIGNS: &str = "+-*/<>=!?$@%_&~^.:#";
const QUOTE_SIGNS: &str = "'`,";

pub type IResult<'a, T, A> = nom::IResult<Input<'a, A>, T, NomError<'a, A>>;

pub fn module<A: Allocator + Clone>(input: Input<'_, A>) -> IResult<Vec<Expression<'_, A>, A>, A> {
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
            map(none_of("\";"), |_| None),
            map(raw_string, |_| None),
            map(comment, Some),
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

fn symbol<'a, A: Allocator + Clone>(input: Input<'a, A>) -> IResult<Expression<'a, A>, A> {
    map(
        token(positioned(take_while1::<_, Input<'a, A>, _>(|character| {
            character.is_alphanumeric() || SYMBOL_SIGNS.contains(character)
        }))),
        |(input, position)| Expression::Symbol(&input, position),
    )(input)
}

fn expression<A: Allocator + Clone>(input: Input<'_, A>) -> IResult<Expression<'_, A>, A> {
    let allocator = input.extra.clone();

    alt((
        context("symbol", symbol),
        context("list", list_like("(", ")", Expression::List)),
        context("string", string),
        context(
            "quote",
            map(
                positioned(tuple((token(recognize(one_of(QUOTE_SIGNS))), expression))),
                move |((sign, expression), position)| {
                    Expression::Quote(&sign, Box::new_in(expression, allocator.clone()), position)
                },
            ),
        ),
        context("vector", list_like("[", "]", Expression::Vector)),
    ))(input)
}

fn list_like<'a, A: Allocator + Clone>(
    left: &'static str,
    right: &'static str,
    create: fn(Vec<Expression<'a, A>, A>, Position) -> Expression<'a, A>,
) -> impl FnMut(Input<'a, A>) -> IResult<Expression<'a, A>, A> {
    move |input| {
        map(
            positioned(preceded(
                sign(left),
                cut(terminated(many0(expression), sign(right))),
            )),
            |(expressions, position)| create(expressions, position),
        )(input)
    }
}

fn string<A: Allocator + Clone>(input: Input<'_, A>) -> IResult<Expression<'_, A>, A> {
    token(raw_string)(input)
}

fn raw_string<A: Allocator + Clone>(input: Input<'_, A>) -> IResult<Expression<'_, A>, A> {
    map(
        positioned(delimited(
            char('"'),
            recognize(many0(alt((
                recognize(none_of("\\\"")),
                tag("\\\\"),
                tag("\\\""),
                tag("\\n"),
                tag("\\r"),
                tag("\\t"),
            )))),
            char('"'),
        )),
        |(input, position)| Expression::String(*input, position),
    )(input)
}

fn sign<'a, A: Allocator + Clone>(sign: &'static str) -> impl Fn(Input<'a, A>) -> IResult<(), A> {
    move |input| value((), token(tag(sign)))(input)
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
                token(nom_locate::position),
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
    map(
        terminated(
            positioned_meta(preceded(char(';'), take_until("\n"))),
            newline,
        ),
        |(input, position)| Comment::new(&input, position),
    )(input)
}

fn hash_directive<A: Allocator + Clone>(input: Input<A>) -> IResult<HashDirective<'_>, A> {
    map(
        terminated(
            positioned_meta(preceded(char('#'), take_until("\n"))),
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
    fn parse_false() {
        assert_eq!(
            expression(Input::new_extra("#f", Global)).unwrap().1,
            Expression::Symbol("#f", Position::new(0, 2))
        );
        assert_eq!(
            expression(Input::new_extra("#false", Global)).unwrap().1,
            Expression::Symbol("#false", Position::new(0, 6))
        );
    }

    #[test]
    fn parse_true() {
        assert_eq!(
            expression(Input::new_extra("#t", Global)).unwrap().1,
            Expression::Symbol("#t", Position::new(0, 2))
        );
        assert_eq!(
            expression(Input::new_extra("#true", Global)).unwrap().1,
            Expression::Symbol("#true", Position::new(0, 5))
        );
    }

    #[test]
    fn parse_list() {
        assert_eq!(
            expression(Input::new_extra("(1 2 3)", Global)).unwrap().1,
            Expression::List(
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
    fn parse_vector() {
        assert_eq!(
            expression(Input::new_extra("[1 2 3]", Global)).unwrap().1,
            Expression::Vector(
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
        fn parse_escaped_characters() {
            assert_eq!(
                string(Input::new_extra("\"\\\\\\n\\r\\t\"", Global))
                    .unwrap()
                    .1,
                Expression::String("\\\\\\n\\r\\t", Position::new(0, 10))
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
                Comment::new("", Position::new(0, 1))
            );
        }

        #[test]
        fn parse_comment() {
            assert_eq!(
                comment(Input::new_extra(";foo\n", Global)).unwrap().1,
                Comment::new("foo", Position::new(0, 4))
            );
        }

        #[test]
        fn parse_comments() {
            assert_eq!(
                comments(Input::new_extra(";foo\n;bar\n", Global))
                    .unwrap()
                    .1,
                vec![
                    Comment::new("foo", Position::new(0, 4)),
                    Comment::new("bar", Position::new(5, 9))
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
                    Comment::new("foo", Position::new(0, 4)),
                    Comment::new("bar", Position::new(6, 10))
                ]
            );
        }
    }
}
