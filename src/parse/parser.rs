use nom::{
    branch::alt,
    bytes::complete::{tag, take_while1},
    character::complete::{char, digit1, multispace0, multispace1},
    combinator::{map, map_res},
    error::{context, ParseError},
    sequence::{delimited, preceded, separated_pair},
    IResult,
};
use nom_locate::LocatedSpan;

type Span<'a> = LocatedSpan<&'a str>;

pub fn symbol<'a, E: ParseError<Span<'a>>>(
    input: Span<'a>,
) -> IResult<Span<'a>, Expression<'a>, E> {
    map(
        take_while1(|c: char| c.is_alphabetic() || "+-*/<>=!?$%_&~^".contains(c)),
        Expression::Symbol,
    )(input)
}

pub fn number<'a, E: ParseError<Span<'a>>>(
    input: Span<'a>,
) -> IResult<Span<'a>, Expression<'a>, E> {
    map_res(digit1, |s: Span<'a>| {
        s.fragment.parse::<i64>().map(Expression::Number)
    })(input)
}

pub fn expression<'a, E: ParseError<Span<'a>>>(
    input: Span<'a>,
) -> IResult<Span<'a>, Expression<'a>, E> {
    alt((
        context("symbol", parse_symbol),
        context("number", parse_number),
        context(
            "list",
            delimited(
                preceded(multispace0, char('(')),
                map(
                    separated_pair(
                        multispace0,
                        parse_expr,
                        delimited(multispace1, parse_expr, multispace0),
                    ),
                    |(first, rest)| {
                        let mut items = vec![first];
                        items.extend(rest);
                        Expression::List(items)
                    },
                ),
                char(')'),
            ),
        ),
    ))(input)
}
