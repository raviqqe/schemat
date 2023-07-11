use crate::ast::Expression;
use nom::{
    branch::alt,
    bytes::complete::take_while1,
    character::complete::{char, multispace0, multispace1},
    combinator::{map, value},
    error::{context, ParseError},
    multi::many0,
    sequence::{delimited, preceded, separated_pair},
    IResult,
};
use nom_locate::LocatedSpan;

const SYMBOL_SIGNS: &str = "+-*/<>=!?$%_&~^";

pub type Span<'a> = LocatedSpan<&'a str>;

pub fn module<'a, E: ParseError<Span<'a>>>(
    input: Span<'a>,
) -> IResult<Span<'a>, Vec<Expression<'a>>, E> {
    many0(expression)(input)
}

fn symbol<'a, E: ParseError<Span<'a>>>(input: Span<'a>) -> IResult<Span<'a>, Expression<'a>, E> {
    map(
        take_while1(|character: char| {
            character.is_alphanumeric() || SYMBOL_SIGNS.contains(character)
        }),
        Expression::Symbol,
    )(input)
}

pub fn expression<'a, E: ParseError<Span<'a>>>(
    input: Span<'a>,
) -> IResult<Span<'a>, Expression<'a>, E> {
    alt((
        context("symbol", symbol),
        context(
            "list",
            delimited(
                sign('('),
                map(
                    separated_pair(
                        multispace0,
                        expression,
                        delimited(multispace1, expression, multispace0),
                    ),
                    |(first, rest)| {
                        let mut items = vec![first];
                        items.extend(rest);
                        Expression::List(items)
                    },
                ),
                sign(')'),
            ),
        ),
    ))(input)
}

fn sign<'a, E: ParseError<Span<'a>>>(
    character: char,
) -> impl Fn(Span<'a>) -> IResult<Span<'a>, (), E> {
    move |input| value((), preceded(multispace0, char(character)))(input)
}
