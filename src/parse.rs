mod parser;

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

// Define the input type
type Span<'a> = LocatedSpan<&'a str>;

// Define the Scheme expression type
#[derive(Debug)]
enum Expr<'a> {
    Symbol(&'a str),
    Number(i64),
    List(Vec<Expr<'a>>),
}

// Parse a Scheme symbol
fn parse_symbol<'a, E>(input: Span<'a>) -> IResult<Span<'a>, Expr<'a>, E>
where
    E: ParseError<Span<'a>>,
{
    map(
        take_while1(|c: char| c.is_alphabetic() || "+-*/<>=!?$%_&~^".contains(c)),
        Expr::Symbol,
    )(input)
}

// Parse a Scheme number
fn parse_number<'a, E>(input: Span<'a>) -> IResult<Span<'a>, Expr<'a>, E>
where
    E: ParseError<Span<'a>>,
{
    map_res(digit1, |s: Span<'a>| {
        s.fragment.parse::<i64>().map(Expr::Number)
    })(input)
}

// Parse a Scheme expression
fn parse_expr<'a, E>(input: Span<'a>) -> IResult<Span<'a>, Expr<'a>, E>
where
    E: ParseError<Span<'a>>,
{
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
                        Expr::List(items)
                    },
                ),
                char(')'),
            ),
        ),
    ))(input)
}

fn main() {
    // Example usage
    let input = Span::new("(+ 1 (* 2 3))");

    match parse_expr::<()>(input) {
        Ok((_, expr)) => println!("Parsed expression: {:?}", expr),
        Err(e) => println!("Error parsing input: {:?}", e),
    }
}
