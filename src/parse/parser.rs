use super::{error::NomError, input::Input};
use crate::{
    ast::{BlockComment, Comment, Expression, HashDirective, LineComment},
    position::Position,
};
use nom::{
    Parser,
    branch::alt,
    bytes::complete::{tag, take_until},
    character::complete::{
        anychar, char, multispace0, multispace1, none_of, one_of, satisfy, space0,
    },
    combinator::{all_consuming, cut, eof, map, not, peek, recognize, value},
    error::context,
    multi::{fold_many0, many0, many0_count, many1_count},
    sequence::{delimited, preceded, terminated},
};

const SYMBOL_SIGNS: &str = "+-*/<>=!?$@%_&~^.:";
const SPECIAL_SIGNS: &str = ";\"";

pub type IResult<'a, T> = nom::IResult<Input<'a>, T, NomError<'a>>;

pub fn module(input: Input) -> IResult<Vec<Expression>> {
    all_consuming(delimited(
        many0_count(hash_directive),
        many0(expression),
        blank,
    ))
    .parse(input)
}

pub fn comments(input: Input) -> IResult<Vec<Comment>> {
    all_consuming(fold_many0(
        alt((
            map(none_of("\"|;#\\"), |_| None),
            map(raw_string, |_| None),
            map(raw_quoted_symbol, |_| None),
            map(raw_symbol, |_| None),
            map(comment, Some),
            map(quote, |_| None),
        )),
        Vec::new,
        |mut all, comment| {
            if let Some(comment) = comment {
                all.push(comment);
            }

            all
        },
    ))
    .parse(input)
}

pub fn hash_directives(input: Input) -> IResult<Vec<HashDirective>> {
    many0(hash_directive).parse(input)
}

fn symbol(input: Input) -> IResult<Expression> {
    map(token(positioned(raw_symbol)), |(input, position)| {
        Expression::Symbol(&input, position)
    })
    .parse(input)
}

fn raw_symbol(input: Input) -> IResult<Input> {
    recognize((head_symbol_character, many0(tail_symbol_character))).parse(input)
}

fn quoted_symbol(input: Input) -> IResult<Expression> {
    map(token(positioned(raw_quoted_symbol)), |(input, position)| {
        Expression::QuotedSymbol(&input, position)
    })
    .parse(input)
}

fn raw_quoted_symbol(input: Input) -> IResult<Input> {
    delimited(
        char('|'),
        recognize(many0(alt((
            recognize(none_of("\\|")),
            tag("\\|"),
            recognize((char('\\'), one_of(SPECIAL_SIGNS))),
            escaped_character,
        )))),
        char('|'),
    )
    .parse(input)
}

fn escaped_character(input: Input) -> IResult<Input> {
    alt((
        tag("\\\\"),
        tag("\\'"),
        tag("\\a"),
        tag("\\b"),
        tag("\\n"),
        tag("\\r"),
        tag("\\t"),
        tag("\\\n"),
        recognize((char('\\'), hexadecimal_digit, hexadecimal_digit)),
        recognize(preceded(
            tag("\\x"),
            cut(terminated(many1_count(hexadecimal_digit), char(';'))),
        )),
        recognize((
            tag("\\u"),
            hexadecimal_digit,
            hexadecimal_digit,
            hexadecimal_digit,
            hexadecimal_digit,
        )),
    ))
    .parse(input)
}

fn head_symbol_character(input: Input) -> IResult<Input> {
    recognize(alt((
        value(
            (),
            satisfy(|character| {
                character.is_ascii_alphanumeric()
                    || SYMBOL_SIGNS.contains(character)
                    || !character.is_ascii() && !character.is_whitespace()
            }),
        ),
        value((), (char('\\'), anychar)),
    )))
    .parse(input)
}

fn tail_symbol_character(input: Input) -> IResult<Input> {
    alt((head_symbol_character, recognize(char('#')))).parse(input)
}

fn expression(input: Input) -> IResult<Expression> {
    alt((
        context("list", list_like("(", ")")),
        context("string", string),
        context(
            "quote",
            map(
                token(positioned((quote, expression))),
                |((sign, expression), position)| {
                    Expression::Quote(&sign, expression.into(), position)
                },
            ),
        ),
        context("quoted symbol", quoted_symbol),
        context("symbol", symbol),
        context("vector", list_like("[", "]")),
        context("map", list_like("{", "}")),
    ))
    .parse(input)
}

fn quote(input: Input) -> IResult<Input> {
    alt((
        tag("'"),
        tag("`"),
        tag(",@"),
        tag(","),
        tag("#;"),
        tag("#"),
        terminated(raw_symbol, peek(not(alt((multispace1, eof))))),
    ))
    .parse(input)
}

fn list_like(left: &'static str, right: &'static str) -> impl FnMut(Input) -> IResult<Expression> {
    move |input| {
        map(
            token(positioned((
                sign(left),
                cut((many0(expression), sign(right))),
            ))),
            |((left, (expressions, right)), position)| {
                Expression::List(&left, &right, expressions, position)
            },
        )
        .parse(input)
    }
}

fn string(input: Input) -> IResult<Expression> {
    map(token(positioned(raw_string)), |(input, position)| {
        Expression::String(*input, position)
    })
    .parse(input)
}

fn raw_string(input: Input) -> IResult<Input> {
    delimited(
        char('"'),
        recognize(many0(alt((
            recognize(none_of("\\\"")),
            tag("\\\""),
            escaped_character,
        )))),
        char('"'),
    )
    .parse(input)
}

fn hexadecimal_digit(input: Input) -> IResult<char> {
    satisfy(|character| character.is_ascii_hexdigit()).parse(input)
}

fn sign(sign: &'static str) -> impl Fn(Input) -> IResult<Input> {
    move |input| token(tag(sign)).parse(input)
}

fn token<'a, T>(
    mut parser: impl Parser<Input<'a>, Output = T, Error = NomError<'a>>,
) -> impl FnMut(Input<'a>) -> IResult<'a, T> {
    move |input| preceded(blank, |input| parser.parse(input)).parse(input)
}

fn positioned<'a, T>(
    mut parser: impl Parser<Input<'a>, Output = T, Error = NomError<'a>>,
) -> impl FnMut(Input<'a>) -> IResult<'a, (T, Position)> {
    move |input| {
        map(
            (
                nom_locate::position,
                |input| parser.parse(input),
                nom_locate::position,
            ),
            |(start, value, end)| {
                (
                    value,
                    Position::new(start.location_offset(), end.location_offset()),
                )
            },
        )
        .parse(input)
    }
}

fn positioned_meta<'a, T>(
    mut parser: impl Parser<Input<'a>, Output = T, Error = NomError<'a>>,
) -> impl FnMut(Input<'a>) -> IResult<'a, (T, Position)> {
    move |input| {
        map(
            (
                preceded(multispace0, nom_locate::position),
                |input| parser.parse(input),
                nom_locate::position,
            ),
            |(start, value, end)| {
                (
                    value,
                    Position::new(start.location_offset(), end.location_offset()),
                )
            },
        )
        .parse(input)
    }
}

fn blank(input: Input) -> IResult<()> {
    value(
        (),
        many0_count(alt((value((), multispace1), value((), comment)))),
    )
    .parse(input)
}

fn comment(input: Input) -> IResult<Comment> {
    alt((
        map(line_comment, From::from),
        map(block_comment, From::from),
    ))
    .parse(input)
}

fn line_comment(input: Input) -> IResult<LineComment> {
    map(
        terminated(
            positioned_meta(preceded(char(';'), take_until("\n"))),
            newline,
        ),
        |(input, position)| LineComment::new(&input, position),
    )
    .parse(input)
}

fn block_comment(input: Input) -> IResult<BlockComment> {
    map(
        positioned_meta(delimited(
            tag("#|"),
            recognize(many0((not(tag("|#")), anychar))),
            tag("|#"),
        )),
        |(input, position)| BlockComment::new(&input, position),
    )
    .parse(input)
}

fn hash_directive(input: Input) -> IResult<HashDirective> {
    map(
        terminated(
            positioned_meta(preceded(
                (char('#'), not(peek(char('|')))),
                take_until("\n"),
            )),
            newline,
        ),
        |(input, position)| HashDirective::new(&input, position),
    )
    .parse(input)
}

fn newline(input: Input) -> IResult<()> {
    value(
        (),
        many1_count(delimited(space0, nom::character::complete::newline, space0)),
    )
    .parse(input)
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn parse_symbol() {
        assert_eq!(
            expression(Input::new("x")).unwrap().1,
            Expression::Symbol("x", Position::new(0, 1))
        );
        assert_eq!(
            expression(Input::new("foo")).unwrap().1,
            Expression::Symbol("foo", Position::new(0, 3))
        );
        assert_eq!(
            expression(Input::new("1")).unwrap().1,
            Expression::Symbol("1", Position::new(0, 1))
        );
        assert_eq!(
            expression(Input::new("42")).unwrap().1,
            Expression::Symbol("42", Position::new(0, 2))
        );
        assert_eq!(
            expression(Input::new("3.14")).unwrap().1,
            Expression::Symbol("3.14", Position::new(0, 4))
        );
        assert_eq!(
            expression(Input::new("a#a")).unwrap().1,
            Expression::Symbol("a#a", Position::new(0, 3))
        );
        assert_eq!(
            expression(Input::new("\\#")).unwrap().1,
            Expression::Symbol("\\#", Position::new(0, 2))
        );
        assert_eq!(
            expression(Input::new("あいうえお")).unwrap().1,
            Expression::Symbol("あいうえお", Position::new(0, 15))
        );
        assert_eq!(
            expression(Input::new("→")).unwrap().1,
            Expression::Symbol("→", Position::new(0, 3))
        );
        assert_eq!(
            expression(Input::new("…")).unwrap().1,
            Expression::Symbol("…", Position::new(0, 3))
        );
        assert_eq!(
            expression(Input::new("🎉")).unwrap().1,
            Expression::Symbol("🎉", Position::new(0, 4))
        );
        assert_eq!(
            expression(Input::new("list→vector")).unwrap().1,
            Expression::Symbol("list→vector", Position::new(0, 13))
        );
        assert_eq!(
            expression(Input::new("foo\u{3000}bar")).unwrap().1,
            Expression::Symbol("foo", Position::new(0, 3))
        );
    }

    #[test]
    fn parse_quoted_symbol() {
        assert_eq!(
            expression(Input::new("|a|")).unwrap().1,
            Expression::QuotedSymbol("a", Position::new(0, 3))
        );
        assert_eq!(
            expression(Input::new("|a b|")).unwrap().1,
            Expression::QuotedSymbol("a b", Position::new(0, 5))
        );
        assert_eq!(
            expression(Input::new("|\\||")).unwrap().1,
            Expression::QuotedSymbol("\\|", Position::new(0, 4))
        );
        assert_eq!(
            expression(Input::new("|\t\n|")).unwrap().1,
            Expression::QuotedSymbol("\t\n", Position::new(0, 4))
        );
        assert_eq!(
            expression(Input::new("|\\t\\n|")).unwrap().1,
            Expression::QuotedSymbol("\\t\\n", Position::new(0, 6))
        );
        assert_eq!(
            expression(Input::new("|\\;|")).unwrap().1,
            Expression::QuotedSymbol("\\;", Position::new(0, 4))
        );
        assert_eq!(
            expression(Input::new("|\\\"|")).unwrap().1,
            Expression::QuotedSymbol("\\\"", Position::new(0, 4))
        );
        assert_eq!(
            expression(Input::new("|\\a\\b|")).unwrap().1,
            Expression::QuotedSymbol("\\a\\b", Position::new(0, 6))
        );
    }

    #[test]
    fn parse_invalid_symbol() {
        assert!(expression(Input::new("#")).is_err());
        assert!(expression(Input::new("\u{3000}")).is_err());
    }

    #[test]
    fn parse_list() {
        assert_eq!(
            expression(Input::new("(1 2 3)")).unwrap().1,
            Expression::List(
                "(",
                ")",
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
    fn parse_list_with_correct_position() {
        assert_eq!(
            expression(Input::new(" ()")).unwrap().1,
            Expression::List("(", ")", vec![], Position::new(1, 3))
        );
    }

    #[test]
    fn parse_character() {
        assert_eq!(
            expression(Input::new("#\\a")).unwrap().1,
            Expression::Quote(
                "#",
                Expression::Symbol("\\a", Position::new(1, 3)).into(),
                Position::new(0, 3)
            )
        );
        assert_eq!(
            expression(Input::new("#\\(")).unwrap().1,
            Expression::Quote(
                "#",
                Expression::Symbol("\\(", Position::new(1, 3)).into(),
                Position::new(0, 3)
            )
        );
        assert_eq!(
            expression(Input::new("#\\;")).unwrap().1,
            Expression::Quote(
                "#",
                Expression::Symbol("\\;", Position::new(1, 3)).into(),
                Position::new(0, 3)
            )
        );
        assert_eq!(
            expression(Input::new("#\\ ")).unwrap().1,
            Expression::Quote(
                "#",
                Expression::Symbol("\\ ", Position::new(1, 3)).into(),
                Position::new(0, 3)
            )
        );
        assert_eq!(
            expression(Input::new("#\\space")).unwrap().1,
            Expression::Quote(
                "#",
                Expression::Symbol("\\space", Position::new(1, 7)).into(),
                Position::new(0, 7)
            )
        );
        assert_eq!(
            expression(Input::new("#\\\n")).unwrap().1,
            Expression::Quote(
                "#",
                Expression::Symbol("\\\n", Position::new(1, 3)).into(),
                Position::new(0, 3)
            )
        );
    }

    #[test]
    fn parse_vector() {
        assert_eq!(
            expression(Input::new("#(1 2 3)")).unwrap().1,
            Expression::Quote(
                "#",
                Expression::List(
                    "(",
                    ")",
                    vec![
                        Expression::Symbol("1", Position::new(2, 3)),
                        Expression::Symbol("2", Position::new(4, 5)),
                        Expression::Symbol("3", Position::new(6, 7))
                    ],
                    Position::new(1, 8)
                )
                .into(),
                Position::new(0, 8)
            )
        );
    }

    #[test]
    fn parse_byte_vector() {
        assert_eq!(
            expression(Input::new("#u8(1 2 3)")).unwrap().1,
            Expression::Quote(
                "#",
                Expression::Quote(
                    "u8",
                    Expression::List(
                        "(",
                        ")",
                        vec![
                            Expression::Symbol("1", Position::new(4, 5)),
                            Expression::Symbol("2", Position::new(6, 7)),
                            Expression::Symbol("3", Position::new(8, 9))
                        ],
                        Position::new(3, 10)
                    )
                    .into(),
                    Position::new(1, 10)
                )
                .into(),
                Position::new(0, 10)
            ),
        );
    }

    #[test]
    fn parse_bracket_vector() {
        assert_eq!(
            expression(Input::new("[1 2 3]")).unwrap().1,
            Expression::List(
                "[",
                "]",
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
    fn parse_map() {
        assert_eq!(
            expression(Input::new("#{1 2 3}")).unwrap().1,
            Expression::Quote(
                "#",
                Expression::List(
                    "{",
                    "}",
                    vec![
                        Expression::Symbol("1", Position::new(2, 3)),
                        Expression::Symbol("2", Position::new(4, 5)),
                        Expression::Symbol("3", Position::new(6, 7))
                    ],
                    Position::new(1, 8)
                )
                .into(),
                Position::new(0, 8)
            )
        );
    }

    #[test]
    fn parse_single_at_mark() {
        assert_eq!(
            expression(Input::new("(@ foo)")).unwrap().1,
            Expression::List(
                "(",
                ")",
                vec![
                    Expression::Symbol("@", Position::new(1, 2)),
                    Expression::Symbol("foo", Position::new(3, 6)),
                ],
                Position::new(0, 7)
            )
        );
    }

    #[test]
    fn parse_double_at_marks() {
        assert_eq!(
            expression(Input::new("(@@ foo)")).unwrap().1,
            Expression::List(
                "(",
                ")",
                vec![
                    Expression::Symbol("@@", Position::new(1, 3)),
                    Expression::Symbol("foo", Position::new(4, 7)),
                ],
                Position::new(0, 8)
            )
        );
    }

    mod boolean {
        use super::*;
        use pretty_assertions::assert_eq;

        #[test]
        fn parse_false() {
            assert_eq!(
                expression(Input::new("#f")).unwrap().1,
                Expression::Quote(
                    "#",
                    Expression::Symbol("f", Position::new(1, 2)).into(),
                    Position::new(0, 2)
                )
            );
            assert_eq!(
                expression(Input::new("#false")).unwrap().1,
                Expression::Quote(
                    "#",
                    Expression::Symbol("false", Position::new(1, 6)).into(),
                    Position::new(0, 6)
                )
            );
        }

        #[test]
        fn parse_true() {
            assert_eq!(
                expression(Input::new("#t")).unwrap().1,
                Expression::Quote(
                    "#",
                    Expression::Symbol("t", Position::new(1, 2)).into(),
                    Position::new(0, 2)
                )
            );
            assert_eq!(
                expression(Input::new("#true")).unwrap().1,
                Expression::Quote(
                    "#",
                    Expression::Symbol("true", Position::new(1, 5)).into(),
                    Position::new(0, 5)
                )
            );
        }

        #[test]
        fn parse_boolean_followed_by_comment() {
            assert_eq!(
                expression(Input::new("#f;")).unwrap().1,
                Expression::Quote(
                    "#",
                    Expression::Symbol("f", Position::new(1, 2)).into(),
                    Position::new(0, 2)
                )
            );
        }

        #[test]
        fn parse_boolean_followed_by_right_parenthesis() {
            assert_eq!(
                expression(Input::new("#f)")).unwrap().1,
                Expression::Quote(
                    "#",
                    Expression::Symbol("f", Position::new(1, 2)).into(),
                    Position::new(0, 2)
                )
            );
        }
    }

    mod quote {
        use super::*;
        use pretty_assertions::assert_eq;

        #[test]
        fn parse_quote() {
            assert_eq!(
                expression(Input::new("'foo")).unwrap().1,
                Expression::Quote(
                    "'",
                    Expression::Symbol("foo", Position::new(1, 4)).into(),
                    Position::new(0, 4)
                )
            );
        }

        #[test]
        fn parse_quote_with_correct_position() {
            assert_eq!(
                expression(Input::new(" 'foo")).unwrap().1,
                Expression::Quote(
                    "'",
                    Expression::Symbol("foo", Position::new(2, 5)).into(),
                    Position::new(1, 5)
                )
            );
        }

        #[test]
        fn parse_unquote() {
            assert_eq!(
                expression(Input::new(",foo")).unwrap().1,
                Expression::Quote(
                    ",",
                    Expression::Symbol("foo", Position::new(1, 4)).into(),
                    Position::new(0, 4)
                )
            );
        }

        #[test]
        fn parse_hash_quote() {
            assert_eq!(
                expression(Input::new("#()")).unwrap().1,
                Expression::Quote(
                    "#",
                    Expression::List("(", ")", vec![], Position::new(1, 3)).into(),
                    Position::new(0, 3)
                )
            );
        }

        #[test]
        fn parse_hash_semicolon_quote() {
            assert_eq!(
                expression(Input::new("#;()")).unwrap().1,
                Expression::Quote(
                    "#;",
                    Expression::List("(", ")", vec![], Position::new(2, 4)).into(),
                    Position::new(0, 4)
                )
            );
        }

        #[test]
        fn parse_quasi_quote() {
            assert_eq!(
                expression(Input::new("`foo")).unwrap().1,
                Expression::Quote(
                    "`",
                    Expression::Symbol("foo", Position::new(1, 4)).into(),
                    Position::new(0, 4)
                )
            );
        }

        #[test]
        fn parse_splicing_unquote() {
            assert_eq!(
                expression(Input::new(",@foo")).unwrap().1,
                Expression::Quote(
                    ",@",
                    Expression::Symbol("foo", Position::new(2, 5)).into(),
                    Position::new(0, 5)
                )
            );
        }

        #[test]
        fn parse_splicing_unquote_with_space() {
            assert_eq!(
                expression(Input::new(",@ foo")).unwrap().1,
                Expression::Quote(
                    ",@",
                    Expression::Symbol("foo", Position::new(3, 6)).into(),
                    Position::new(0, 6)
                )
            );
        }

        #[test]
        fn parse_symbol_and_quoted_list() {
            assert_eq!(
                (expression, expression)
                    .parse(Input::new("#u8 ()"))
                    .unwrap()
                    .1,
                (
                    Expression::Quote(
                        "#",
                        Expression::Symbol("u8", Position::new(1, 3)).into(),
                        Position::new(0, 3)
                    ),
                    Expression::List("(", ")", vec![], Position::new(4, 6))
                )
            );
        }

        #[test]
        fn parse_quote_with_space() {
            assert_eq!(
                expression(Input::new("' foo")).unwrap().1,
                Expression::Quote(
                    "'",
                    Expression::Symbol("foo", Position::new(2, 5)).into(),
                    Position::new(0, 5)
                )
            );
        }
    }

    mod hash_directive {
        use super::*;
        use pretty_assertions::assert_eq;

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

        #[test]
        fn parse_comment() {
            assert_eq!(hash_directives(Input::new("#||#\n")).unwrap().1, vec![]);
        }
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
        fn parse_escaped_double_quote() {
            assert_eq!(
                string(Input::new("\"\\\"\"")).unwrap().1,
                Expression::String("\\\"", Position::new(0, 4))
            );
        }

        #[test]
        fn parse_escaped_single_quote() {
            assert_eq!(
                string(Input::new("\"\\'\"")).unwrap().1,
                Expression::String("\\'", Position::new(0, 4))
            );
        }

        #[test]
        fn parse_escaped_characters() {
            assert_eq!(
                string(Input::new("\"\\\\\\a\\b\\n\\r\\t\"")).unwrap().1,
                Expression::String("\\\\\\a\\b\\n\\r\\t", Position::new(0, 14))
            );
        }

        #[test]
        fn parse_scheme_hexadecimal_bytes() {
            assert_eq!(
                string(Input::new("\"\\x0F;\"")).unwrap().1,
                Expression::String("\\x0F;", Position::new(0, 7))
            );
            assert_eq!(
                string(Input::new("\"\\xABCD;\"")).unwrap().1,
                Expression::String("\\xABCD;", Position::new(0, 9))
            );
        }

        #[test]
        fn parse_scheme_hexadecimal_scalar_values() {
            assert_eq!(
                string(Input::new("\"\\xA;\"")).unwrap().1,
                Expression::String("\\xA;", Position::new(0, 6))
            );
            assert_eq!(
                string(Input::new("\"\\x3bb;\"")).unwrap().1,
                Expression::String("\\x3bb;", Position::new(0, 8))
            );
            assert_eq!(
                string(Input::new("\"\\x1F600;\"")).unwrap().1,
                Expression::String("\\x1F600;", Position::new(0, 10))
            );
        }

        #[test]
        fn parse_invalid_scheme_hexadecimal_scalar_values() {
            assert!(string(Input::new("\"\\x;\"")).is_err());
            assert!(string(Input::new("\"\\x41\"")).is_err());
            assert!(string(Input::new("\"\\xG;\"")).is_err());
        }

        // https://webassembly.github.io/spec/core/text/values.html#strings
        #[test]
        fn parse_wasm_hexadecimal_bytes() {
            assert_eq!(
                string(Input::new("\"\\00\\FF\"")).unwrap().1,
                Expression::String("\\00\\FF", Position::new(0, 8))
            );
        }

        #[test]
        fn parse_multi_line() {
            assert_eq!(
                string(Input::new("\"a\\\nb\"")).unwrap().1,
                Expression::String("a\\\nb", Position::new(0, 6))
            );
        }

        #[test]
        fn parse_escaped_unicode() {
            assert_eq!(
                string(Input::new("\"\\ubeef\"")).unwrap().1,
                Expression::String("\\ubeef", Position::new(0, 8))
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
                LineComment::new("", Position::new(0, 1)).into()
            );
        }

        #[test]
        fn parse_comment() {
            assert_eq!(
                comment(Input::new(";foo\n")).unwrap().1,
                LineComment::new("foo", Position::new(0, 4)).into()
            );
        }

        #[test]
        fn parse_comments() {
            assert_eq!(
                comments(Input::new(";foo\n;bar\n")).unwrap().1,
                vec![
                    LineComment::new("foo", Position::new(0, 4)).into(),
                    LineComment::new("bar", Position::new(5, 9)).into()
                ]
            );
        }

        #[test]
        fn parse_comments_with_blank_lines() {
            assert_eq!(
                comments(Input::new(";foo\n\n;bar\n")).unwrap().1,
                vec![
                    LineComment::new("foo", Position::new(0, 4)).into(),
                    LineComment::new("bar", Position::new(6, 10)).into()
                ]
            );
        }

        #[test]
        fn parse_comments_skipping_hash_semicolon() {
            assert_eq!(
                comments(Input::new("#;foo\n;bar\n")).unwrap().1,
                vec![LineComment::new("bar", Position::new(6, 10)).into()]
            );
        }

        #[test]
        fn parse_comments_skipping_hash_character() {
            assert_eq!(
                comments(Input::new("#foo\n;bar\n")).unwrap().1,
                vec![LineComment::new("bar", Position::new(5, 9)).into()]
            );
        }

        #[test]
        fn parse_comment_character() {
            assert_eq!(comments(Input::new("#\\;foo\n")).unwrap().1, vec![]);
        }

        #[test]
        fn parse_comment_in_list() {
            assert_eq!(
                comments(Input::new("(f\n;foo\nx)")).unwrap().1,
                vec![LineComment::new("foo", Position::new(3, 7)).into()]
            );
        }

        #[test]
        fn parse_comment_with_vector() {
            assert_eq!(comments(Input::new("#()")).unwrap().1, vec![]);
        }

        mod block {
            use super::*;
            use pretty_assertions::assert_eq;

            #[test]
            fn parse_empty() {
                assert_eq!(
                    block_comment(Input::new("#||#")).unwrap().1,
                    BlockComment::new("", Position::new(0, 4))
                );
            }

            #[test]
            fn parse_one_line() {
                assert_eq!(
                    block_comment(Input::new("#|foo|#")).unwrap().1,
                    BlockComment::new("foo", Position::new(0, 7))
                );
            }

            #[test]
            fn parse_multi_line() {
                assert_eq!(
                    // spell-checker: disable-next-line
                    block_comment(Input::new("#|\nfoo\nbar\nbaz\n|#"))
                        .unwrap()
                        .1,
                    // spell-checker: disable-next-line
                    BlockComment::new("\nfoo\nbar\nbaz\n", Position::new(0, 17))
                );
            }

            #[test]
            fn parse_in_comments() {
                assert_eq!(
                    comments(Input::new("#|foo|#")).unwrap().1,
                    vec![BlockComment::new("foo", Position::new(0, 7)).into()]
                );
            }
        }
    }
}
