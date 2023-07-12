use super::{error::NomError, input::Input};
use crate::{ast::Expression, position::Position};
use nom::{
    branch::alt,
    bytes::complete::{tag, take_until, take_while1},
    character::complete::{char, multispace1, none_of, space0},
    combinator::{all_consuming, cut, map, recognize, value},
    error::context,
    multi::{many0, many1},
    sequence::{delimited, preceded, terminated, tuple},
    Parser,
};

const SYMBOL_SIGNS: &str = "+-*/<>=!?$@%_&~^.:#";

pub type IResult<'a, T> = nom::IResult<Input<'a>, T, NomError<'a>>;

pub fn module(input: Input<'_>) -> IResult<Vec<Expression<'_>>> {
    all_consuming(delimited(many0(hash_line), many0(expression), blank))(input)
}

pub fn comments(input: Input) -> IResult<Vec<Comment>> {
    map(
        all_consuming(many0(alt((
            map(comment, Some),
            map(raw_string, |_| None),
            map(none_of("\"#"), |_| None),
        )))),
        |comments| comments.into_iter().flat_map(|comment| comment).collect(),
    )(input)
}

fn symbol<'a>(input: Input<'a>) -> IResult<Expression<'a>> {
    map(
        token(positioned(take_while1::<_, Input<'a>, _>(|character| {
            character.is_alphanumeric() || SYMBOL_SIGNS.contains(character)
        }))),
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
                positioned(preceded(
                    sign('('),
                    cut(terminated(many0(expression), sign(')'))),
                )),
                |(expressions, position)| Expression::List(expressions, position),
            ),
        ),
    ))(input)
}

fn string(input: Input<'_>) -> IResult<Expression<'_>> {
    token(raw_string)
}

fn raw_string(input: Input<'_>) -> IResult<Expression<'_>> {
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

fn sign<'a>(character: char) -> impl Fn(Input<'a>) -> IResult<()> {
    move |input| value((), token(char(character)))(input)
}

fn token<'a, T>(
    mut parser: impl Parser<Input<'a>, T, NomError<'a>>,
) -> impl FnMut(Input<'a>) -> IResult<T> {
    move |input| preceded(blank, |input| parser.parse(input))(input)
}

fn positioned<'a, T>(
    mut parser: impl Parser<Input<'a>, T, NomError<'a>>,
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
    value((), many0(alt((multispace1, comment))))(input)
}

fn comment(input: Input<'_>) -> IResult<Input<'_>> {
    delimited(char(';'), take_until("\n"), newline)(input)
}

fn hash_line(input: Input<'_>) -> IResult<Input<'_>> {
    delimited(char('#'), take_until("\n"), newline)(input)
}

fn newline(input: Input<'_>) -> IResult<()> {
    value(
        (),
        many1(delimited(space0, nom::character::complete::newline, space0)),
    )(input)
}

#[cfg(test)]
mod tests {
    use super::*;

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
