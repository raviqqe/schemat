use super::{error::NomError, input::Input};
use crate::{
    ast::{Comment, Expression, HashDirective},
    position::Position,
};
use nom::{
    branch::alt,
    bytes::complete::{tag, take_until, take_while1},
    character::complete::{char, multispace0, multispace1, none_of, space0},
    combinator::{all_consuming, cut, map, recognize, value},
    error::context,
    multi::{fold_many0, many0, many0_count, many1},
    sequence::{delimited, preceded, terminated, tuple},
    Parser,
};
use smallvec::SmallVec;
use std::alloc::Allocator;

const BUFFER_SIZE: usize = 128;

const SYMBOL_SIGNS: &str = "+-*/<>=!?$@%_&~^.:#";

pub type IResult<'a, T, A: Allocator + Clone> = nom::IResult<Input<'a, A>, T, NomError<'a, A>>;

pub fn module<A: Allocator + Clone>(input: Input<'_, A>) -> IResult<Vec<Expression<'_, A>, A>, A> {
    all_consuming(delimited(many0(hash_directive), many0(expression), blank))(input)
}

pub fn comments<A: Allocator + Clone>(input: Input<A>) -> IResult<Vec<Comment, A>, A> {
    map(
        all_consuming(fold_many0(
            alt((
                map(comment, Some),
                map(raw_string, |_| None),
                map(none_of("\";"), |_| None),
            )),
            SmallVec::<[Option<Comment>; BUFFER_SIZE]>::new,
            |mut all, x| {
                all.push(x);
                all
            },
        )),
        |comments| {
            let vec = Vec::new_in(input.extra());
            vec.extend(comments.into_iter().flatten());
            vec
        },
    )(input)
}

pub fn hash_directives<A: Allocator + Clone>(input: Input<A>) -> IResult<Vec<HashDirective, A>, A> {
    all_consuming(terminated(
        many0(hash_directive),
        tuple((many0_count(expression), blank)),
    ))(input)
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
    alt((
        context("symbol", symbol),
        context(
            "quote",
            map(
                positioned(preceded(sign('\''), expression)),
                |(expression, position)| {
                    Expression::Quote(Box::new_in(expression, input.extra), position)
                },
            ),
        ),
        context("string", string),
        context(
            "list",
            map(
                positioned(preceded(
                    sign('('),
                    cut(terminated(
                        fold_many0(
                            expression,
                            || Vec::new_in(input.extra),
                            |mut all, expression| {
                                all.push(expression);
                                all
                            },
                        ),
                        sign(')'),
                    )),
                )),
                |(expressions, position)| Expression::List(expressions, position),
            ),
        ),
    ))(input)
}

fn string<A: Allocator + Clone>(input: Input<'_, A>) -> IResult<Expression<'_, A>, A> {
    token(raw_string)(input)
}

fn raw_string<A: Allocator + Clone>(input: Input<'_, A>) -> IResult<Expression<'_, A>, A> {
    map(
        positioned(delimited(
            char('"'),
            recognize(fold_many0(
                alt((
                    recognize(none_of("\\\"")),
                    tag("\\\\"),
                    tag("\\\""),
                    tag("\\n"),
                    tag("\\r"),
                    tag("\\t"),
                )),
                || Vec::new_in(input.extra),
                |mut all, expression| {
                    all.push(expression);
                    all
                },
            )),
            char('"'),
        )),
        |(input, position)| Expression::String(*input, position),
    )(input)
}

fn sign<'a, A: Allocator + Clone>(character: char) -> impl Fn(Input<'a, A>) -> IResult<(), A> {
    move |input| value((), token(char(character)))(input)
}

fn token<'a, T>(
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
    value((), many0(alt((value((), multispace1), value((), comment)))))(input)
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
        many1(delimited(space0, nom::character::complete::newline, space0)),
    )(input)
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn parse_false() {
        assert_eq!(
            expression(Input::new("#f")).unwrap().1,
            Expression::Symbol("#f", Position::new(0, 2))
        );
        assert_eq!(
            expression(Input::new("#false")).unwrap().1,
            Expression::Symbol("#false", Position::new(0, 6))
        );
    }

    #[test]
    fn parse_true() {
        assert_eq!(
            expression(Input::new("#t")).unwrap().1,
            Expression::Symbol("#t", Position::new(0, 2))
        );
        assert_eq!(
            expression(Input::new("#true")).unwrap().1,
            Expression::Symbol("#true", Position::new(0, 5))
        );
    }

    #[test]
    fn parse_shebang() {
        assert_eq!(
            hash_directive(Input::new("#!/bin/sh\n")).unwrap().1,
            HashDirective::new("!/bin/sh", Position::new(0, 9))
        );
    }

    #[test]
    fn parse_lang_directive() {
        assert_eq!(
            hash_directive(Input::new("#lang r7rs\n")).unwrap().1,
            HashDirective::new("lang r7rs", Position::new(0, 10))
        );
    }

    mod string {
        use super::*;
        use pretty_assertions::assert_eq;

        #[test]
        fn parse_empty() {
            assert_eq!(
                string(Input::new("\"\"")).unwrap().1,
                Expression::String("", Position::new(0, 2))
            );
        }

        #[test]
        fn parse_non_empty() {
            assert_eq!(
                string(Input::new("\"foo\"")).unwrap().1,
                Expression::String("foo", Position::new(0, 5))
            );
        }

        #[test]
        fn parse_escaped_characters() {
            assert_eq!(
                string(Input::new("\"\\\\\\n\\r\\t\"")).unwrap().1,
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
                comment(Input::new(";\n")).unwrap().1,
                Comment::new("", Position::new(0, 1))
            );
        }

        #[test]
        fn parse_comment() {
            assert_eq!(
                comment(Input::new(";foo\n")).unwrap().1,
                Comment::new("foo", Position::new(0, 4))
            );
        }

        #[test]
        fn parse_comments() {
            assert_eq!(
                comments(Input::new(";foo\n;bar\n")).unwrap().1,
                vec![
                    Comment::new("foo", Position::new(0, 4)),
                    Comment::new("bar", Position::new(5, 9))
                ]
            );
        }

        #[test]
        fn parse_comments_with_blank_lines() {
            assert_eq!(
                comments(Input::new(";foo\n\n;bar\n")).unwrap().1,
                vec![
                    Comment::new("foo", Position::new(0, 4)),
                    Comment::new("bar", Position::new(6, 10))
                ]
            );
        }
    }
}
