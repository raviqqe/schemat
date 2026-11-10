use super::input::Input;
use crate::{
    ast::{BlockComment, Comment, Expression, HashDirective, LineComment},
    position::Position,
};
use winnow::{
    Parser,
    ascii::{multispace0, multispace1, space0},
    combinator::{alt, cut_err, delimited, not, peek, preceded, repeat, terminated},
    error::{EmptyError, ErrMode},
    token::{any, none_of, one_of, rest, take_until},
};

const SYMBOL_SIGNS: &str = "+-*/<>=!?$@%_&~^.:";
const SYMBOL_END_HINT: &str = ")]}";
const SPECIAL_SIGNS: &str = ";\"";

pub type Error = ErrMode<EmptyError>;

type Result<T> = core::result::Result<T, Error>;

pub fn module<'a>(input: &mut Input<'a>) -> Result<Vec<Expression<'a>>> {
    delimited(many0_count(hash_directive), repeat(0.., expression), blank0).parse_next(input)
}

pub fn comments<'a>(input: &mut Input<'a>) -> Result<Vec<Comment<'a>>> {
    repeat(
        0..,
        alt((
            none_of(['"', '|', ';', '#', '\\']).map(|_| None),
            raw_string.map(|_| None),
            raw_quoted_symbol.map(|_| None),
            raw_symbol.map(|_| None),
            comment.map(Some),
            quote.map(|_| None),
        )),
    )
    .fold(Vec::new, |mut all, comment| {
        if let Some(comment) = comment {
            all.push(comment);
        }

        all
    })
    .parse_next(input)
}

pub fn hash_directives<'a>(input: &mut Input<'a>) -> Result<Vec<HashDirective<'a>>> {
    terminated(repeat(0.., hash_directive), rest).parse_next(input)
}

fn symbol<'a>(input: &mut Input<'a>) -> Result<Expression<'a>> {
    token(positioned(raw_symbol))
        .map(|(symbol, position)| Expression::Symbol(symbol, position))
        .parse_next(input)
}

fn raw_symbol<'a>(input: &mut Input<'a>) -> Result<&'a str> {
    (head_symbol_character, many0_count(tail_symbol_character))
        .take()
        .parse_next(input)
}

fn quoted_symbol<'a>(input: &mut Input<'a>) -> Result<Expression<'a>> {
    token(positioned(raw_quoted_symbol))
        .map(|(symbol, position)| Expression::QuotedSymbol(symbol, position))
        .parse_next(input)
}

fn raw_quoted_symbol<'a>(input: &mut Input<'a>) -> Result<&'a str> {
    delimited(
        '|',
        many0_count(alt((
            none_of(['\\', '|']).take(),
            "\\|",
            ('\\', one_of(|character| SPECIAL_SIGNS.contains(character))).take(),
            escaped_character,
        )))
        .take(),
        '|',
    )
    .parse_next(input)
}

fn escaped_character<'a>(input: &mut Input<'a>) -> Result<&'a str> {
    alt((
        alt(["\\\\", "\\'", "\\a", "\\b", "\\n", "\\r", "\\t", "\\\n"]),
        ('\\', hexadecimal_digit, hexadecimal_digit).take(),
        preceded(
            "\\x",
            cut_err(terminated(many1_count(hexadecimal_digit), ';')),
        )
        .take(),
        (
            "\\u",
            hexadecimal_digit,
            hexadecimal_digit,
            hexadecimal_digit,
            hexadecimal_digit,
        )
            .take(),
    ))
    .parse_next(input)
}

fn head_symbol_character<'a>(input: &mut Input<'a>) -> Result<&'a str> {
    alt((
        one_of(|character: char| {
            character.is_ascii_alphanumeric()
                || SYMBOL_SIGNS.contains(character)
                || !character.is_ascii() && !character.is_whitespace()
        })
        .void(),
        ('\\', any).void(),
    ))
    .take()
    .parse_next(input)
}

fn tail_symbol_character<'a>(input: &mut Input<'a>) -> Result<&'a str> {
    alt((head_symbol_character, "#")).parse_next(input)
}

fn expression<'a>(input: &mut Input<'a>) -> Result<Expression<'a>> {
    alt((
        list_like("(", ")").context("list"),
        string.context("string"),
        token(positioned((quote, expression)))
            .map(|((sign, expression), position)| {
                Expression::Quote(sign, expression.into(), position)
            })
            .context("quote"),
        quoted_symbol.context("quoted symbol"),
        symbol.context("symbol"),
        list_like("[", "]").context("vector"),
        list_like("{", "}").context("map"),
    ))
    .parse_next(input)
}

fn quote<'a>(input: &mut Input<'a>) -> Result<&'a str> {
    alt((
        "'",
        "`",
        ",@",
        ",",
        "#;",
        "#",
        terminated(
            raw_symbol,
            peek(not(alt((
                blank,
                one_of(|character| SYMBOL_END_HINT.contains(character)).void(),
            )))),
        ),
    ))
    .parse_next(input)
}

fn list_like<'a>(
    left: &'static str,
    right: &'static str,
) -> impl Parser<Input<'a>, Expression<'a>, Error> {
    token(positioned((
        sign(left),
        cut_err((repeat(0.., expression), sign(right))),
    )))
    .map(|((left, (expressions, right)), position)| {
        Expression::List(left, right, expressions, position)
    })
}

fn string<'a>(input: &mut Input<'a>) -> Result<Expression<'a>> {
    token(positioned(raw_string))
        .map(|(string, position)| Expression::String(string, position))
        .parse_next(input)
}

fn raw_string<'a>(input: &mut Input<'a>) -> Result<&'a str> {
    delimited(
        '"',
        many0_count(alt((
            none_of(['\\', '"']).take(),
            "\\\"",
            escaped_character,
        )))
        .take(),
        '"',
    )
    .parse_next(input)
}

fn hexadecimal_digit(input: &mut Input) -> Result<char> {
    one_of(|character: char| character.is_ascii_hexdigit()).parse_next(input)
}

fn sign<'a>(sign: &'static str) -> impl Parser<Input<'a>, &'a str, Error> {
    token(sign)
}

fn token<'a, T>(parser: impl Parser<Input<'a>, T, Error>) -> impl Parser<Input<'a>, T, Error> {
    preceded(blank0, parser)
}

fn positioned<'a, T>(
    parser: impl Parser<Input<'a>, T, Error>,
) -> impl Parser<Input<'a>, (T, Position), Error> {
    parser
        .with_span()
        .map(|(value, range)| (value, Position::new(range.start, range.end)))
}

fn positioned_meta<'a, T>(
    parser: impl Parser<Input<'a>, T, Error>,
) -> impl Parser<Input<'a>, (T, Position), Error> {
    preceded(multispace0, positioned(parser))
}

fn many0_count<'a, T>(
    parser: impl Parser<Input<'a>, T, Error>,
) -> impl Parser<Input<'a>, (), Error> {
    repeat(0.., parser)
}

fn many1_count<'a, T>(
    parser: impl Parser<Input<'a>, T, Error>,
) -> impl Parser<Input<'a>, (), Error> {
    repeat(1.., parser)
}

fn blank0(input: &mut Input) -> Result<()> {
    many0_count(blank).parse_next(input)
}

fn blank(input: &mut Input) -> Result<()> {
    alt((multispace1.void(), comment.void())).parse_next(input)
}

fn comment<'a>(input: &mut Input<'a>) -> Result<Comment<'a>> {
    alt((line_comment.map(From::from), block_comment.map(From::from))).parse_next(input)
}

fn line_comment<'a>(input: &mut Input<'a>) -> Result<LineComment<'a>> {
    terminated(
        positioned_meta(preceded(';', take_until(0.., "\n"))),
        newline,
    )
    .map(|(comment, position)| LineComment::new(comment, position))
    .parse_next(input)
}

fn block_comment<'a>(input: &mut Input<'a>) -> Result<BlockComment<'a>> {
    positioned_meta(delimited("#|", many0_count((not("|#"), any)).take(), "|#"))
        .map(|(comment, position)| BlockComment::new(comment, position))
        .parse_next(input)
}

fn hash_directive<'a>(input: &mut Input<'a>) -> Result<HashDirective<'a>> {
    terminated(
        positioned_meta(preceded(('#', not(peek('|'))), take_until(0.., "\n"))),
        newline,
    )
    .map(|(directive, position)| HashDirective::new(directive, position))
    .parse_next(input)
}

fn newline(input: &mut Input) -> Result<()> {
    many1_count(delimited(space0, winnow::ascii::newline, space0)).parse_next(input)
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn parse_symbol() {
        assert_eq!(
            expression(&mut Input::new("x")).unwrap(),
            Expression::Symbol("x", Position::new(0, 1))
        );
        assert_eq!(
            expression(&mut Input::new("foo")).unwrap(),
            Expression::Symbol("foo", Position::new(0, 3))
        );
        assert_eq!(
            expression(&mut Input::new("1")).unwrap(),
            Expression::Symbol("1", Position::new(0, 1))
        );
        assert_eq!(
            expression(&mut Input::new("42")).unwrap(),
            Expression::Symbol("42", Position::new(0, 2))
        );
        assert_eq!(
            expression(&mut Input::new("3.14")).unwrap(),
            Expression::Symbol("3.14", Position::new(0, 4))
        );
        assert_eq!(
            expression(&mut Input::new("a#a")).unwrap(),
            Expression::Symbol("a#a", Position::new(0, 3))
        );
        assert_eq!(
            expression(&mut Input::new("\\#")).unwrap(),
            Expression::Symbol("\\#", Position::new(0, 2))
        );
        assert_eq!(
            expression(&mut Input::new("あいうえお")).unwrap(),
            Expression::Symbol("あいうえお", Position::new(0, 15))
        );
        assert_eq!(
            expression(&mut Input::new("→")).unwrap(),
            Expression::Symbol("→", Position::new(0, 3))
        );
        assert_eq!(
            expression(&mut Input::new("…")).unwrap(),
            Expression::Symbol("…", Position::new(0, 3))
        );
        assert_eq!(
            expression(&mut Input::new("🎉")).unwrap(),
            Expression::Symbol("🎉", Position::new(0, 4))
        );
        assert_eq!(
            expression(&mut Input::new("list→vector")).unwrap(),
            Expression::Symbol("list→vector", Position::new(0, 13))
        );
        assert_eq!(
            expression(&mut Input::new("foo\u{3000}bar")).unwrap(),
            Expression::Symbol("foo", Position::new(0, 3))
        );
    }

    #[test]
    fn parse_quoted_symbol() {
        assert_eq!(
            expression(&mut Input::new("|a|")).unwrap(),
            Expression::QuotedSymbol("a", Position::new(0, 3))
        );
        assert_eq!(
            expression(&mut Input::new("|a b|")).unwrap(),
            Expression::QuotedSymbol("a b", Position::new(0, 5))
        );
        assert_eq!(
            expression(&mut Input::new("|\\||")).unwrap(),
            Expression::QuotedSymbol("\\|", Position::new(0, 4))
        );
        assert_eq!(
            expression(&mut Input::new("|\t\n|")).unwrap(),
            Expression::QuotedSymbol("\t\n", Position::new(0, 4))
        );
        assert_eq!(
            expression(&mut Input::new("|\\t\\n|")).unwrap(),
            Expression::QuotedSymbol("\\t\\n", Position::new(0, 6))
        );
        assert_eq!(
            expression(&mut Input::new("|\\;|")).unwrap(),
            Expression::QuotedSymbol("\\;", Position::new(0, 4))
        );
        assert_eq!(
            expression(&mut Input::new("|\\\"|")).unwrap(),
            Expression::QuotedSymbol("\\\"", Position::new(0, 4))
        );
        assert_eq!(
            expression(&mut Input::new("|\\a\\b|")).unwrap(),
            Expression::QuotedSymbol("\\a\\b", Position::new(0, 6))
        );
    }

    #[test]
    fn parse_invalid_symbol() {
        assert!(expression(&mut Input::new("#")).is_err());
        assert!(expression(&mut Input::new("\u{3000}")).is_err());
    }

    #[test]
    fn parse_list() {
        assert_eq!(
            expression(&mut Input::new("(1 2 3)")).unwrap(),
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
            expression(&mut Input::new(" ()")).unwrap(),
            Expression::List("(", ")", vec![], Position::new(1, 3))
        );
    }

    #[test]
    fn parse_character() {
        assert_eq!(
            expression(&mut Input::new("#\\a")).unwrap(),
            Expression::Quote(
                "#",
                Expression::Symbol("\\a", Position::new(1, 3)).into(),
                Position::new(0, 3)
            )
        );
        assert_eq!(
            expression(&mut Input::new("#\\(")).unwrap(),
            Expression::Quote(
                "#",
                Expression::Symbol("\\(", Position::new(1, 3)).into(),
                Position::new(0, 3)
            )
        );
        assert_eq!(
            expression(&mut Input::new("#\\;")).unwrap(),
            Expression::Quote(
                "#",
                Expression::Symbol("\\;", Position::new(1, 3)).into(),
                Position::new(0, 3)
            )
        );
        assert_eq!(
            expression(&mut Input::new("#\\ ")).unwrap(),
            Expression::Quote(
                "#",
                Expression::Symbol("\\ ", Position::new(1, 3)).into(),
                Position::new(0, 3)
            )
        );
        assert_eq!(
            expression(&mut Input::new("#\\space")).unwrap(),
            Expression::Quote(
                "#",
                Expression::Symbol("\\space", Position::new(1, 7)).into(),
                Position::new(0, 7)
            )
        );
        assert_eq!(
            expression(&mut Input::new("#\\\n")).unwrap(),
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
            expression(&mut Input::new("#(1 2 3)")).unwrap(),
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
            expression(&mut Input::new("#u8(1 2 3)")).unwrap(),
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
            expression(&mut Input::new("[1 2 3]")).unwrap(),
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
            expression(&mut Input::new("#{1 2 3}")).unwrap(),
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
            expression(&mut Input::new("(@ foo)")).unwrap(),
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
            expression(&mut Input::new("(@@ foo)")).unwrap(),
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
                expression(&mut Input::new("#f")).unwrap(),
                Expression::Quote(
                    "#",
                    Expression::Symbol("f", Position::new(1, 2)).into(),
                    Position::new(0, 2)
                )
            );
            assert_eq!(
                expression(&mut Input::new("#false")).unwrap(),
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
                expression(&mut Input::new("#t")).unwrap(),
                Expression::Quote(
                    "#",
                    Expression::Symbol("t", Position::new(1, 2)).into(),
                    Position::new(0, 2)
                )
            );
            assert_eq!(
                expression(&mut Input::new("#true")).unwrap(),
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
                expression(&mut Input::new("#f;")).unwrap(),
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
                expression(&mut Input::new("#f)")).unwrap(),
                Expression::Quote(
                    "#",
                    Expression::Symbol("f", Position::new(1, 2)).into(),
                    Position::new(0, 2)
                )
            );
        }
    }

    mod comment {
        use super::*;
        use pretty_assertions::assert_eq;

        #[test]
        fn parse_empty() {
            assert_eq!(
                comment(&mut Input::new(";\n")).unwrap(),
                LineComment::new("", Position::new(0, 1)).into()
            );
        }

        #[test]
        fn parse_comment() {
            assert_eq!(
                comment(&mut Input::new(";foo\n")).unwrap(),
                LineComment::new("foo", Position::new(0, 4)).into()
            );
        }

        #[test]
        fn parse_comments() {
            assert_eq!(
                comments(&mut Input::new(";foo\n;bar\n")).unwrap(),
                vec![
                    LineComment::new("foo", Position::new(0, 4)).into(),
                    LineComment::new("bar", Position::new(5, 9)).into()
                ]
            );
        }

        #[test]
        fn parse_comments_with_blank_lines() {
            assert_eq!(
                comments(&mut Input::new(";foo\n\n;bar\n")).unwrap(),
                vec![
                    LineComment::new("foo", Position::new(0, 4)).into(),
                    LineComment::new("bar", Position::new(6, 10)).into()
                ]
            );
        }

        #[test]
        fn parse_comments_skipping_hash_semicolon() {
            assert_eq!(
                comments(&mut Input::new("#;foo\n;bar\n")).unwrap(),
                vec![LineComment::new("bar", Position::new(6, 10)).into()]
            );
        }

        #[test]
        fn parse_comments_skipping_hash_character() {
            assert_eq!(
                comments(&mut Input::new("#foo\n;bar\n")).unwrap(),
                vec![LineComment::new("bar", Position::new(5, 9)).into()]
            );
        }

        #[test]
        fn parse_comment_character() {
            assert_eq!(comments(&mut Input::new("#\\;foo\n")).unwrap(), vec![]);
        }

        #[test]
        fn parse_comment_in_list() {
            assert_eq!(
                comments(&mut Input::new("(f\n;foo\nx)")).unwrap(),
                vec![LineComment::new("foo", Position::new(3, 7)).into()]
            );
        }

        #[test]
        fn parse_comment_with_vector() {
            assert_eq!(comments(&mut Input::new("#()")).unwrap(), vec![]);
        }

        mod block {
            use super::*;
            use pretty_assertions::assert_eq;

            #[test]
            fn parse_empty() {
                assert_eq!(
                    block_comment(&mut Input::new("#||#")).unwrap(),
                    BlockComment::new("", Position::new(0, 4))
                );
            }

            #[test]
            fn parse_one_line() {
                assert_eq!(
                    block_comment(&mut Input::new("#|foo|#")).unwrap(),
                    BlockComment::new("foo", Position::new(0, 7))
                );
            }

            #[test]
            fn parse_multi_line() {
                assert_eq!(
                    // spell-checker: disable-next-line
                    block_comment(&mut Input::new("#|\nfoo\nbar\nbaz\n|#")).unwrap(),
                    // spell-checker: disable-next-line
                    BlockComment::new("\nfoo\nbar\nbaz\n", Position::new(0, 17))
                );
            }

            #[test]
            fn parse_in_comments() {
                assert_eq!(
                    comments(&mut Input::new("#|foo|#")).unwrap(),
                    vec![BlockComment::new("foo", Position::new(0, 7)).into()]
                );
            }
        }
    }

    mod hash_directive {
        use super::*;
        use pretty_assertions::assert_eq;

        #[test]
        fn parse_shebang() {
            assert_eq!(
                hash_directive(&mut Input::new("#!/bin/sh\n")).unwrap(),
                HashDirective::new("!/bin/sh", Position::new(0, 9))
            );
        }

        #[test]
        fn parse_lang_directive() {
            assert_eq!(
                hash_directive(&mut Input::new("#lang r7rs\n")).unwrap(),
                HashDirective::new("lang r7rs", Position::new(0, 10))
            );
        }

        #[test]
        fn parse_comment() {
            assert_eq!(hash_directives(&mut Input::new("#||#\n")).unwrap(), vec![]);
        }
    }

    mod quote {
        use super::*;
        use pretty_assertions::assert_eq;

        #[test]
        fn parse_quote() {
            assert_eq!(
                expression(&mut Input::new("'foo")).unwrap(),
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
                expression(&mut Input::new(" 'foo")).unwrap(),
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
                expression(&mut Input::new(",foo")).unwrap(),
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
                expression(&mut Input::new("#()")).unwrap(),
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
                expression(&mut Input::new("#;()")).unwrap(),
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
                expression(&mut Input::new("`foo")).unwrap(),
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
                expression(&mut Input::new(",@foo")).unwrap(),
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
                expression(&mut Input::new(",@ foo")).unwrap(),
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
                    .parse_next(&mut Input::new("#u8 ()"))
                    .unwrap(),
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
                expression(&mut Input::new("' foo")).unwrap(),
                Expression::Quote(
                    "'",
                    Expression::Symbol("foo", Position::new(2, 5)).into(),
                    Position::new(0, 5)
                )
            );
        }

        #[test]
        fn parse_symbol_quoting_adjacent_list() {
            assert_eq!(
                expression(&mut Input::new("foo(bar)")).unwrap(),
                Expression::Quote(
                    "foo",
                    Expression::List(
                        "(",
                        ")",
                        vec![Expression::Symbol("bar", Position::new(4, 7))],
                        Position::new(3, 8)
                    )
                    .into(),
                    Position::new(0, 8)
                )
            );
        }

        #[test]
        fn parse_symbol_and_comment() {
            assert_eq!(
                (expression, expression)
                    .parse_next(&mut Input::new("foo;bar\nbaz"))
                    .unwrap(),
                (
                    Expression::Symbol("foo", Position::new(0, 3)),
                    Expression::Symbol("baz", Position::new(8, 11))
                )
            );
        }

        #[test]
        fn parse_hash_symbol_and_comment() {
            assert_eq!(
                (expression, expression)
                    .parse_next(&mut Input::new("#t;foo\nbar"))
                    .unwrap(),
                (
                    Expression::Quote(
                        "#",
                        Expression::Symbol("t", Position::new(1, 2)).into(),
                        Position::new(0, 2)
                    ),
                    Expression::Symbol("bar", Position::new(7, 10))
                )
            );
        }

        #[test]
        fn parse_hash_symbol_quoting_string() {
            assert_eq!(
                expression(&mut Input::new("#rx\"foo\"")).unwrap(),
                Expression::Quote(
                    "#",
                    Expression::Quote(
                        "rx",
                        Expression::String("foo", Position::new(3, 8)).into(),
                        Position::new(1, 8)
                    )
                    .into(),
                    Position::new(0, 8)
                )
            );
        }

        #[test]
        fn fail_to_parse_hash_symbol_quoting_unclosed_list() {
            assert!(matches!(
                expression(&mut Input::new("#u8(1")),
                Err(ErrMode::Cut(_))
            ));
        }
    }

    mod racket {
        use super::*;
        use pretty_assertions::assert_eq;

        #[test]
        fn parse_command_without_arguments() {
            assert_eq!(
                expression(&mut Input::new("@foo")).unwrap(),
                Expression::Symbol("@foo", Position::new(0, 4))
            );
        }

        #[test]
        fn parse_command_with_datum() {
            assert_eq!(
                expression(&mut Input::new("@foo[bar]")).unwrap(),
                Expression::Quote(
                    "@foo",
                    Expression::List(
                        "[",
                        "]",
                        vec![Expression::Symbol("bar", Position::new(5, 8))],
                        Position::new(4, 9)
                    )
                    .into(),
                    Position::new(0, 9)
                )
            );
        }

        #[test]
        fn parse_command_with_text_body() {
            assert_eq!(
                expression(&mut Input::new("@foo{bar}")).unwrap(),
                Expression::Quote(
                    "@foo",
                    Expression::List(
                        "{",
                        "}",
                        vec![Expression::Symbol("bar", Position::new(5, 8))],
                        Position::new(4, 9)
                    )
                    .into(),
                    Position::new(0, 9)
                )
            );
        }

        #[test]
        fn parse_command_and_separated_text_body() {
            assert_eq!(
                (expression, expression)
                    .parse_next(&mut Input::new("@foo {bar}"))
                    .unwrap(),
                (
                    Expression::Symbol("@foo", Position::new(0, 4)),
                    Expression::List(
                        "{",
                        "}",
                        vec![Expression::Symbol("bar", Position::new(6, 9))],
                        Position::new(5, 10)
                    )
                )
            );
        }

        #[test]
        fn parse_text_body_without_command() {
            assert_eq!(
                expression(&mut Input::new("@{foo}")).unwrap(),
                Expression::Quote(
                    "@",
                    Expression::List(
                        "{",
                        "}",
                        vec![Expression::Symbol("foo", Position::new(2, 5))],
                        Position::new(1, 6)
                    )
                    .into(),
                    Position::new(0, 6)
                )
            );
        }

        #[test]
        fn parse_s_expression_command() {
            assert_eq!(
                expression(&mut Input::new("@(foo bar)")).unwrap(),
                Expression::Quote(
                    "@",
                    Expression::List(
                        "(",
                        ")",
                        vec![
                            Expression::Symbol("foo", Position::new(2, 5)),
                            Expression::Symbol("bar", Position::new(6, 9))
                        ],
                        Position::new(1, 10)
                    )
                    .into(),
                    Position::new(0, 10)
                )
            );
        }

        #[test]
        fn parse_string_command() {
            assert_eq!(
                expression(&mut Input::new("@\"foo\"")).unwrap(),
                Expression::Quote(
                    "@",
                    Expression::String("foo", Position::new(1, 6)).into(),
                    Position::new(0, 6)
                )
            );
        }

        #[test]
        fn parse_escaped_identifier() {
            assert_eq!(
                expression(&mut Input::new("@|foo|")).unwrap(),
                Expression::Quote(
                    "@",
                    Expression::QuotedSymbol("foo", Position::new(1, 6)).into(),
                    Position::new(0, 6)
                )
            );
        }

        #[test]
        fn parse_quoted_command() {
            assert_eq!(
                expression(&mut Input::new("@'foo{bar}")).unwrap(),
                Expression::Quote(
                    "@",
                    Expression::Quote(
                        "'",
                        Expression::Quote(
                            "foo",
                            Expression::List(
                                "{",
                                "}",
                                vec![Expression::Symbol("bar", Position::new(6, 9))],
                                Position::new(5, 10)
                            )
                            .into(),
                            Position::new(2, 10)
                        )
                        .into(),
                        Position::new(1, 10)
                    )
                    .into(),
                    Position::new(0, 10)
                )
            );
        }

        #[test]
        fn parse_nested_command() {
            assert_eq!(
                expression(&mut Input::new("@foo{bar @baz{qux}}")).unwrap(),
                Expression::Quote(
                    "@foo",
                    Expression::List(
                        "{",
                        "}",
                        vec![
                            Expression::Symbol("bar", Position::new(5, 8)),
                            Expression::Quote(
                                "@baz",
                                Expression::List(
                                    "{",
                                    "}",
                                    vec![Expression::Symbol("qux", Position::new(14, 17))],
                                    Position::new(13, 18)
                                )
                                .into(),
                                Position::new(9, 18)
                            )
                        ],
                        Position::new(4, 19)
                    )
                    .into(),
                    Position::new(0, 19)
                )
            );
        }

        #[test]
        fn parse_command_after_text() {
            assert_eq!(
                expression(&mut Input::new("@foo{R@sup{2}}")).unwrap(),
                Expression::Quote(
                    "@foo",
                    Expression::List(
                        "{",
                        "}",
                        vec![Expression::Quote(
                            "R@sup",
                            Expression::List(
                                "{",
                                "}",
                                vec![Expression::Symbol("2", Position::new(11, 12))],
                                Position::new(10, 13)
                            )
                            .into(),
                            Position::new(5, 13)
                        )],
                        Position::new(4, 14)
                    )
                    .into(),
                    Position::new(0, 14)
                )
            );
        }
    }

    mod string {
        use super::*;
        use pretty_assertions::assert_eq;

        #[test]
        fn parse_empty() {
            assert_eq!(
                string(&mut Input::new("\"\"")).unwrap(),
                Expression::String("", Position::new(0, 2))
            );
        }

        #[test]
        fn parse_non_empty() {
            assert_eq!(
                string(&mut Input::new("\"foo\"")).unwrap(),
                Expression::String("foo", Position::new(0, 5))
            );
        }

        #[test]
        fn parse_escaped_double_quote() {
            assert_eq!(
                string(&mut Input::new("\"\\\"\"")).unwrap(),
                Expression::String("\\\"", Position::new(0, 4))
            );
        }

        #[test]
        fn parse_escaped_single_quote() {
            assert_eq!(
                string(&mut Input::new("\"\\'\"")).unwrap(),
                Expression::String("\\'", Position::new(0, 4))
            );
        }

        #[test]
        fn parse_escaped_characters() {
            assert_eq!(
                string(&mut Input::new("\"\\\\\\a\\b\\n\\r\\t\"")).unwrap(),
                Expression::String("\\\\\\a\\b\\n\\r\\t", Position::new(0, 14))
            );
        }

        #[test]
        fn parse_scheme_hexadecimal_bytes() {
            assert_eq!(
                string(&mut Input::new("\"\\x0F;\"")).unwrap(),
                Expression::String("\\x0F;", Position::new(0, 7))
            );
            assert_eq!(
                string(&mut Input::new("\"\\xABCD;\"")).unwrap(),
                Expression::String("\\xABCD;", Position::new(0, 9))
            );
        }

        #[test]
        fn parse_scheme_hexadecimal_scalar_values() {
            assert_eq!(
                string(&mut Input::new("\"\\xA;\"")).unwrap(),
                Expression::String("\\xA;", Position::new(0, 6))
            );
            assert_eq!(
                string(&mut Input::new("\"\\x3bb;\"")).unwrap(),
                Expression::String("\\x3bb;", Position::new(0, 8))
            );
            assert_eq!(
                string(&mut Input::new("\"\\x1F600;\"")).unwrap(),
                Expression::String("\\x1F600;", Position::new(0, 10))
            );
        }

        #[test]
        fn parse_invalid_scheme_hexadecimal_scalar_values() {
            assert!(string(&mut Input::new("\"\\x;\"")).is_err());
            assert!(string(&mut Input::new("\"\\x41\"")).is_err());
            assert!(string(&mut Input::new("\"\\xG;\"")).is_err());
        }

        // https://webassembly.github.io/spec/core/text/values.html#strings
        #[test]
        fn parse_wasm_hexadecimal_bytes() {
            assert_eq!(
                string(&mut Input::new("\"\\00\\FF\"")).unwrap(),
                Expression::String("\\00\\FF", Position::new(0, 8))
            );
        }

        #[test]
        fn parse_multi_line() {
            assert_eq!(
                string(&mut Input::new("\"a\\\nb\"")).unwrap(),
                Expression::String("a\\\nb", Position::new(0, 6))
            );
        }

        #[test]
        fn parse_escaped_unicode() {
            assert_eq!(
                string(&mut Input::new("\"\\ubeef\"")).unwrap(),
                Expression::String("\\ubeef", Position::new(0, 8))
            );
        }
    }
}
