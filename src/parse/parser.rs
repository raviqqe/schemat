use super::input::Input;
use crate::ast::Expression;
use nom::{
    branch::alt,
    bytes::complete::take_while1,
    character::complete::{char, multispace0},
    combinator::{map, value},
    error::{context, VerboseError},
    multi::many0,
    sequence::{delimited, preceded},
};

const SYMBOL_SIGNS: &str = "+-*/<>=!?$%_&~^";

pub type Error<'a> = VerboseError<Input<'a>>;

pub type IResult<'a, T> = nom::IResult<Input<'a>, T, Error<'a>>;

pub fn module<'a>(input: Input<'a>) -> IResult<Vec<Expression<'a>>> {
    many0(expression)(input)
}

fn symbol<'a>(input: Input<'a>) -> IResult<Expression<'a>> {
    map(
        take_while1::<_, Input<'a>, _>(|character: char| {
            character.is_alphanumeric() || SYMBOL_SIGNS.contains(character)
        }),
        |input| Expression::Symbol(&input),
    )(input)
}

fn expression<'a>(input: Input<'a>) -> IResult<Expression<'a>> {
    alt((
        context("symbol", symbol),
        context(
            "list",
            delimited(
                sign('('),
                map(many0(expression), Expression::List),
                sign(')'),
            ),
        ),
    ))(input)
}

fn sign<'a>(character: char) -> impl Fn(Input<'a>) -> IResult<()> {
    move |input| value((), preceded(multispace0, char(character)))(input)
}

fn token<'a, T>(parser: impl Fn(Input<'a>) -> IResult<T>) -> impl Fn(Input<'a>) -> IResult<T> {
    move |input| preceded(multispace0, parser)(input)
}
