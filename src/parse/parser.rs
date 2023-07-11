use super::{error::Error, input::Input};
use crate::{ast::Expression, position::Position};
use nom::{
    branch::alt,
    bytes::complete::{tag, take_until, take_while1},
    character::complete::{char, multispace1, none_of},
    combinator::{all_consuming, map, recognize, value},
    error::context,
    multi::many0,
    sequence::{delimited, preceded, terminated, tuple},
    Parser,
};

const SYMBOL_SIGNS: &str = "+-*/<>=!?$%_&~^";

pub type IResult<'a, T> = nom::IResult<Input<'a>, T, Error<'a>>;

pub fn module(input: Input<'_>) -> IResult<Vec<Expression<'_>>> {
    all_consuming(terminated(many0(expression), blank))(input)
}

fn symbol<'a>(input: Input<'a>) -> IResult<Expression<'a>> {
    map(
        token(positioned(take_while1::<_, Input<'a>, _>(
            |character: char| character.is_alphanumeric() || SYMBOL_SIGNS.contains(character),
        ))),
        |(input, position)| Expression::Symbol(&input, position),
    )(input)
}

fn expression(input: Input<'_>) -> IResult<Expression<'_>> {
    alt((
        context("symbol", symbol),
        context(
            "quote",
            map(
                positioned(preceded(sign('\''), expression)),
                |(expression, position)| Expression::Quote(expression.into(), position),
            ),
        ),
        context("string", string),
        context(
            "list",
            map(
                positioned(delimited(sign('('), many0(expression), sign(')'))),
                |(expressions, position)| Expression::List(expressions, position),
            ),
        ),
    ))(input)
}

fn string<'a>(input: Input<'a>) -> IResult<Expression<'a>> {
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
        |(input, position): (Input<'a>, _)| Expression::String(*input, position),
    )(input)
}

fn sign<'a>(character: char) -> impl Fn(Input<'a>) -> IResult<()> {
    move |input| value((), token(char(character)))(input)
}

fn token<'a, T>(
    mut parser: impl Parser<Input<'a>, T, Error<'a>>,
) -> impl FnMut(Input<'a>) -> IResult<T> {
    move |input| preceded(blank, |input| parser.parse(input))(input)
}

fn positioned<'a, T>(
    mut parser: impl Parser<Input<'a>, T, Error<'a>>,
) -> impl FnMut(Input<'a>) -> IResult<'a, (T, Position)> {
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

fn blank(input: Input<'_>) -> IResult<()> {
    value((), many0(alt((multispace1, comment, hash_line))))(input)
}

fn comment(input: Input<'_>) -> IResult<Input<'_>> {
    preceded(char(';'), take_until("\n"))(input)
}

fn hash_line(input: Input<'_>) -> IResult<Input<'_>> {
    preceded(char('#'), take_until("\n"))(input)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_shebang() {
        assert_eq!(*hash_line(Input::new("#!/bin/sh\n")).unwrap().1, "!/bin/sh");
    }

    #[test]
    fn parse_lang_directive() {
        assert_eq!(
            *hash_line(Input::new("#lang r7rs\n")).unwrap().1,
            "lang r7rs"
        );
    }

    mod string {
        use super::*;

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

        #[test]
        fn parse_empty() {
            assert_eq!(*comment(Input::new(";\n")).unwrap().1, "");
        }

        #[test]
        fn parse_comment() {
            assert_eq!(*comment(Input::new(";foo\n")).unwrap().1, "foo");
        }
    }
}
