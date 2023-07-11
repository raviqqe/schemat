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
use nom_locate::LocatedSpan;

const SYMBOL_SIGNS: &str = "+-*/<>=!?$%_&~^";

pub type Span<'a> = LocatedSpan<&'a str>;

pub type Error<'a> = VerboseError<Span<'a>>;

pub type IResult<'a, T> = nom::IResult<Span<'a>, T, Error<'a>>;

pub fn module<'a>(input: Span<'a>) -> IResult<Vec<Expression<'a>>> {
    many0(expression)(input)
}

fn symbol<'a>(input: Span<'a>) -> IResult<Expression<'a>> {
    map(
        take_while1(|character: char| {
            character.is_alphanumeric() || SYMBOL_SIGNS.contains(character)
        }),
        Expression::Symbol,
    )(input)
}

fn expression<'a>(input: Span<'a>) -> IResult<Expression<'a>> {
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

fn sign<'a>(character: char) -> impl Fn(Span<'a>) -> IResult<()> {
    move |input| value((), preceded(multispace0, char(character)))(input)
}
