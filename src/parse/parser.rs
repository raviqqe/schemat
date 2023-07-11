use super::{error::Error, input::Input};
use crate::ast::Expression;
use nom::{
    branch::alt,
    bytes::complete::tag,
    bytes::complete::{take_until, take_while1},
    character::complete::{char, multispace1, none_of},
    combinator::{all_consuming, map, recognize, value},
    error::context,
    multi::many0,
    sequence::{delimited, preceded, terminated},
    Parser,
};

const SYMBOL_SIGNS: &str = "+-*/<>=!?$%_&~^";

pub type IResult<'a, T> = nom::IResult<Input<'a>, T, Error<'a>>;

pub fn module<'a>(input: Input<'a>) -> IResult<Vec<Expression<'a>>> {
    all_consuming(terminated(many0(expression), blank))(input)
}

fn symbol<'a>(input: Input<'a>) -> IResult<Expression<'a>> {
    map(
        token(take_while1::<_, Input<'a>, _>(|character: char| {
            character.is_alphanumeric() || SYMBOL_SIGNS.contains(character)
        })),
        |input| Expression::Symbol(&input),
    )(input)
}

fn expression<'a>(input: Input<'a>) -> IResult<Expression<'a>> {
    alt((
        context("symbol", symbol),
        context(
            "quote",
            map(preceded(sign('\''), expression), |expression| {
                Expression::Quote(expression.into())
            }),
        ),
        context("string", string),
        context(
            "list",
            map(
                delimited(sign('('), many0(expression), sign(')')),
                Expression::List,
            ),
        ),
    ))(input)
}

fn string<'a>(input: Input<'a>) -> IResult<Expression<'a>> {
    map(
        delimited(
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
        ),
        |input: Input<'a>| Expression::String(*input),
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

fn blank<'a>(input: Input<'a>) -> IResult<()> {
    value((), many0(alt((multispace1, comment, hash_line))))(input)
}

fn comment<'a>(input: Input<'a>) -> IResult<Input<'a>> {
    preceded(char(';'), take_until("\n"))(input)
}

fn hash_line<'a>(input: Input<'a>) -> IResult<Input<'a>> {
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
                Expression::String("")
            );
        }

        #[test]
        fn parse_non_empty() {
            assert_eq!(
                string(Input::new("\"foo\"")).unwrap().1,
                Expression::String("foo")
            );
        }

        #[test]
        fn parse_escaped_characters() {
            assert_eq!(
                string(Input::new("\"\\\\\\n\\r\\t\"")).unwrap().1,
                Expression::String("\\\\\\n\\r\\t")
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
