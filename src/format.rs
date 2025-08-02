#![expect(unstable_name_collisions)]

use crate::{
    ast::{Comment, Expression, HashDirective},
    context::Context,
    position::Position,
    position_map::PositionMap,
};
use allocator_api2::{alloc::Allocator, vec::Vec};
use core::fmt;
use itertools::Itertools;
use mfmt::{Builder, Document, FormatOptions, empty, line, sequence, utility::is_empty};

const COMMENT_PREFIX: &str = ";";
const QUOTE_SIGNS: &[&str] = &["'", "`", "#"];
const UNQUOTE_SIGNS: &[&str] = &[","];

pub fn format<A: Allocator + Clone>(
    module: &[Expression<A>],
    comments: &[Comment],
    hash_directives: &[HashDirective],
    position_map: &PositionMap,
    allocator: A,
) -> Result<String, fmt::Error> {
    let mut string = Default::default();
    let document = compile_module(
        &mut Context::new(comments, position_map, Builder::new(allocator)),
        module,
        hash_directives,
    );

    mfmt::format(
        &if is_empty(&document) {
            line()
        } else {
            document
        },
        &mut string,
        FormatOptions::new(2),
    )?;

    Ok(string)
}

fn compile_module<'a, A: Allocator + Clone + 'a>(
    context: &mut Context<'a, A>,
    module: &'a [Expression<'a, A>],
    hash_directives: &[HashDirective],
) -> Document<'a> {
    [
        if hash_directives.is_empty() {
            empty()
        } else {
            context.builder().sequence([context.builder().sequence(
                hash_directives
                    .iter()
                    .map(|directive| compile_hash_directive(context, directive)),
            )])
        },
        {
            let expressions = compile_expressions(context, module, false);

            if is_empty(&expressions) {
                empty()
            } else {
                context.builder().sequence([expressions, line()])
            }
        },
        compile_remaining_block_comment(context),
    ]
    .into_iter()
    .fold(empty(), |all, document| {
        if is_empty(&document) {
            all
        } else {
            context.builder().sequence([
                if is_empty(&all) {
                    empty()
                } else {
                    context.builder().sequence([all, line()])
                },
                document,
            ])
        }
    })
}

fn compile_hash_directive<'a, A: Allocator + Clone + 'a>(
    context: &Context<A>,
    hash_directive: &HashDirective,
) -> Document<'a> {
    context.builder().sequence([
        context.builder().strings(["#", hash_directive.value()]),
        line(),
    ])
}

fn compile_expression<'a, A: Allocator + Clone + 'a>(
    context: &mut Context<'a, A>,
    expression: &'a Expression<'a, A>,
    data: bool,
) -> Document<'a> {
    compile_comment(context, expression.position(), |context| match expression {
        Expression::List(left, right, expressions, position) => {
            compile_list(context, expressions, position, left, right, data)
        }
        Expression::Quote(sign, expression, _) => context.builder().clone().sequence([
            (*sign).into(),
            compile_expression(
                context,
                expression,
                QUOTE_SIGNS.contains(sign) || !UNQUOTE_SIGNS.contains(sign) && data,
            ),
        ]),
        Expression::QuotedSymbol(symbol, _) => context.builder().sequence(["|", *symbol, "|"]),
        Expression::String(string, _) => context.builder().sequence(["\"", *string, "\""]),
        Expression::Symbol(name, _) => (*name).into(),
    })
}

fn compile_list<'a, A: Allocator + Clone + 'a>(
    context: &mut Context<'a, A>,
    expressions: &'a [Expression<'a, A>],
    position: &Position,
    left: &'a str,
    right: &'a str,
    data: bool,
) -> Document<'a> {
    let index = line_index(context, position.start());

    let index = expressions
        .iter()
        .position(|expression| line_index(context, expression.position().start()) > index)
        .unwrap_or(expressions.len());
    let first = &expressions[..index];
    let last = &expressions[index..];

    let builder = context.builder().clone();

    builder.sequence([
        compile_comment(
            context,
            &position.set_end(position.start() + left.len()),
            |_| left.into(),
        ),
        builder.indent(
            builder.offside(
                builder.sequence(
                    [builder.flatten(compile_expressions(context, first, data))]
                        .into_iter()
                        .chain(match (first.last(), last.first()) {
                            (Some(first), Some(last)) if line_gap(context, first, last) > 1 => {
                                Some(line())
                            }
                            _ => None,
                        })
                        .chain(if last.is_empty() {
                            None
                        } else {
                            Some(
                                builder.r#break(
                                    builder.sequence([
                                        line(),
                                        compile_expressions(context, last, data),
                                    ]),
                                ),
                            )
                        })
                        .chain({
                            let position = position.set_start(position.end() - right.len());
                            let block_comment = compile_block_comment(context, &position);
                            let inline_comment = compile_inline_comment(context, &position);
                            let block_comment_empty = is_empty(&block_comment);

                            [builder.r#break(builder.sequence([
                                if block_comment_empty { empty() } else { line() },
                                block_comment,
                                if block_comment_empty
                                    && !is_empty(&inline_comment)
                                    && Some(line_index(context, position.start()))
                                        == expressions.last().map(|expression| {
                                            line_index(context, expression.position().end())
                                        })
                                {
                                    " ".into()
                                } else {
                                    empty()
                                },
                                inline_comment,
                                right.into(),
                            ]))]
                        }),
                ),
                !data,
            ),
        ),
    ])
}

fn compile_expressions<'a, A: Allocator + Clone + 'a>(
    context: &mut Context<'a, A>,
    expressions: &'a [Expression<'a, A>],
    data: bool,
) -> Document<'a> {
    let mut documents =
        Vec::with_capacity_in(2 * expressions.len(), context.builder().allocator().clone());
    let mut last_expression = None;

    for expression in expressions {
        if let Some(last_expression) = last_expression {
            documents.push(line());

            if line_gap(context, last_expression, expression) > 1 {
                documents.push(line());
            }
        }

        documents.push(compile_expression(context, expression, data));

        last_expression = Some(expression);
    }

    sequence(documents.leak())
}

fn compile_comment<'a, A: Allocator + Clone + 'a>(
    context: &mut Context<'a, A>,
    position: &Position,
    document: impl Fn(&mut Context<'a, A>) -> Document<'a>,
) -> Document<'a> {
    let block_comment = compile_block_comment(context, position);
    let inline_comment = compile_inline_comment(context, position);
    let inline_space = if is_empty(&inline_comment) {
        empty()
    } else {
        " ".into()
    };
    let document = document(context);
    let suffix_comment = compile_suffix_comment(context, position);

    context.builder().sequence([
        block_comment,
        inline_comment,
        inline_space,
        document,
        suffix_comment,
    ])
}

fn compile_inline_comment<'a, A: Allocator + Clone + 'a>(
    context: &mut Context<'a, A>,
    position: &Position,
) -> Document<'a> {
    let builder = context.builder().clone();

    builder.sequence(
        context
            .drain_inline_comments(position)
            .map(|comment| builder.sequence(["#|", comment.content(), "|#"]))
            .intersperse(" ".into()),
    )
}

fn compile_suffix_comment<'a, A: Allocator + Clone + 'a>(
    context: &mut Context<A>,
    position: &Position,
) -> Document<'a> {
    let builder = context.builder().clone();

    builder.sequence(
        context
            .drain_current_line_comment(line_index(context, position.start()))
            .map(|comment| {
                builder.line_suffixes([" ", COMMENT_PREFIX, comment.content().trim_end()])
            }),
    )
}

fn compile_block_comment<'a, A: Allocator + Clone + 'a>(
    context: &mut Context<'a, A>,
    position: &Position,
) -> Document<'a> {
    let builder = context.builder().clone();
    let comments = builder
        .allocate_slice(context.drain_multi_line_comments(line_index(context, position.start())));

    compile_all_comments(
        context,
        comments,
        Some(line_index(context, position.start())),
    )
}

fn compile_remaining_block_comment<'a, A: Allocator + Clone + 'a>(
    context: &mut Context<'a, A>,
) -> Document<'a> {
    let builder = context.builder().clone();
    let comments = builder.allocate_slice(context.drain_multi_line_comments(usize::MAX));

    compile_all_comments(context, comments, None)
}

fn compile_all_comments<'a, A: Allocator + Clone + 'a>(
    context: &Context<A>,
    comments: &'a [&'a Comment<'a>],
    last_line_index: Option<usize>,
) -> Document<'a> {
    context.builder().sequence(
        comments
            .iter()
            .zip(
                comments
                    .iter()
                    .skip(1)
                    .map(|comment| line_index(context, comment.position().start()))
                    .chain([last_line_index.unwrap_or(0)]),
            )
            .map(|(comment, next_line_index)| match comment {
                Comment::Block(comment) => context.builder().sequence([
                    "#|".into(),
                    line(),
                    comment.content().trim().into(),
                    line(),
                    "|#".into(),
                    line(),
                    if line_index(context, comment.position().end() - 1) + 1 < next_line_index {
                        line()
                    } else {
                        empty()
                    },
                ]),
                Comment::Line(comment) => context.builder().sequence([
                    COMMENT_PREFIX.into(),
                    comment.content().trim_end().into(),
                    context.builder().r#break(line()),
                    if line_index(context, comment.position().end() - 1) + 1 < next_line_index {
                        line()
                    } else {
                        empty()
                    },
                ]),
            }),
    )
}

fn line_gap<A: Allocator + Clone>(
    context: &Context<A>,
    last_expression: &Expression<A>,
    expression: &Expression<A>,
) -> usize {
    let index = line_index(context, expression.position().start());

    context
        .peek_comments(index)
        .next()
        .map(|comment| line_index(context, comment.position().start()))
        .unwrap_or(index)
        .saturating_sub(line_index(context, last_expression.position().end() - 1))
}

fn line_index<A: Allocator + Clone>(context: &Context<A>, offset: usize) -> usize {
    context
        .position_map()
        .line_index(offset)
        .expect("valid offset")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        ast::{BlockComment, LineComment},
        position::Position,
        position_map::PositionMap,
    };
    use allocator_api2::{alloc::Global, vec};
    use indoc::indoc;
    use pretty_assertions::assert_eq;

    #[test]
    fn format_empty() {
        assert_eq!(
            format(&[], &[], &[], &PositionMap::new("\n"), Global).unwrap(),
            "\n"
        );
    }

    #[test]
    fn format_list() {
        assert_eq!(
            format(
                &[Expression::List(
                    "(",
                    ")",
                    vec![
                        Expression::Symbol("foo", Position::new(0, 2)),
                        Expression::Symbol("bar", Position::new(0, 2))
                    ],
                    Position::new(0, 2)
                )],
                &[],
                &[],
                &PositionMap::new("(foo bar)"),
                Global,
            )
            .unwrap(),
            "(foo bar)\n"
        );
    }

    #[test]
    fn format_list_with_split_lines() {
        assert_eq!(
            format(
                &[Expression::List(
                    "(",
                    ")",
                    vec![
                        Expression::Symbol("foo", Position::new(1, 4)),
                        Expression::Symbol("bar", Position::new(5, 8))
                    ],
                    Position::new(0, 9)
                )],
                &[],
                &[],
                &PositionMap::new("(foo\nbar)"),
                Global,
            )
            .unwrap(),
            indoc!(
                "
                (foo
                  bar)
                "
            )
        );
    }

    #[test]
    fn format_list_with_split_lines_and_multiple_elements() {
        assert_eq!(
            format(
                &[Expression::List(
                    "(",
                    ")",
                    vec![
                        Expression::Symbol("foo", Position::new(0, 1)),
                        Expression::Symbol("bar", Position::new(0, 1)),
                        Expression::Symbol("baz", Position::new(2, 3)),
                        Expression::Symbol("qux", Position::new(2, 3)),
                    ],
                    Position::new(0, 1)
                )],
                &[],
                &[],
                &PositionMap::new("a\nb"),
                Global,
            )
            .unwrap(),
            indoc!(
                "
                (foo bar
                  baz
                  qux)
                "
            )
        );
    }

    #[test]
    fn format_quote() {
        assert_eq!(
            format(
                &[Expression::Quote(
                    "'",
                    Expression::Symbol("foo", Position::new(0, 3)).into(),
                    Position::new(0, 3)
                )],
                &[],
                &[],
                &PositionMap::new("'foo"),
                Global,
            )
            .unwrap(),
            "'foo\n"
        );
    }

    #[test]
    fn format_unquote() {
        assert_eq!(
            format(
                &[Expression::Quote(
                    ",",
                    Expression::Symbol("foo", Position::new(0, 3)).into(),
                    Position::new(0, 3)
                )],
                &[],
                &[],
                &PositionMap::new("'foo"),
                Global,
            )
            .unwrap(),
            ",foo\n"
        );
    }

    #[test]
    fn format_string() {
        assert_eq!(
            format(
                &[Expression::String("foo", Position::new(0, 3))],
                &[],
                &[],
                &PositionMap::new("\"foo\""),
                Global,
            )
            .unwrap(),
            "\"foo\"\n"
        );
    }

    #[test]
    fn format_multi_line_string() {
        assert_eq!(
            format(
                &[Expression::String("a\\\nb", Position::new(0, 6))],
                &[],
                &[],
                &PositionMap::new("\"a\\\nb\""),
                Global,
            )
            .unwrap(),
            "\"a\\\nb\"\n"
        );
    }

    #[test]
    fn format_symbol() {
        assert_eq!(
            format(
                &[Expression::Symbol("foo", Position::new(0, 3))],
                &[],
                &[],
                &PositionMap::new("foo"),
                Global,
            )
            .unwrap(),
            "foo\n"
        );
    }

    #[test]
    fn format_quoted_symbol() {
        assert_eq!(
            format(
                &[Expression::QuotedSymbol("foo", Position::new(0, 3))],
                &[],
                &[],
                &PositionMap::new("foo"),
                Global,
            )
            .unwrap(),
            "|foo|\n"
        );
    }

    #[test]
    fn format_vector() {
        assert_eq!(
            format(
                &[Expression::List(
                    "[",
                    "]",
                    vec![
                        Expression::Symbol("foo", Position::new(0, 2)),
                        Expression::Symbol("bar", Position::new(0, 2))
                    ],
                    Position::new(0, 2)
                )],
                &[],
                &[],
                &PositionMap::new("[foo bar]"),
                Global,
            )
            .unwrap(),
            "[foo bar]\n"
        );
    }

    mod list {
        use super::*;
        use allocator_api2::vec;
        use pretty_assertions::assert_eq;

        #[test]
        fn double_indent_of_nested_list() {
            assert_eq!(
                format(
                    &[Expression::List(
                        "(",
                        ")",
                        vec![Expression::List(
                            "(",
                            ")",
                            vec![
                                Expression::Symbol("foo", Position::new(0, 1)),
                                Expression::Symbol("bar", Position::new(1, 2))
                            ],
                            Position::new(0, 1)
                        )],
                        Position::new(0, 1)
                    )],
                    &[],
                    &[],
                    &PositionMap::new("\n\n\na"),
                    Global,
                )
                .unwrap(),
                indoc!(
                    "
                    ((foo
                        bar))
                    "
                )
            );
        }

        #[test]
        fn format_blank_line() {
            assert_eq!(
                format(
                    &[Expression::List(
                        "(",
                        ")",
                        vec![
                            Expression::Symbol("foo", Position::new(0, 1)),
                            Expression::Symbol("bar", Position::new(1, 2))
                        ],
                        Position::new(0, 1)
                    )],
                    &[],
                    &[],
                    &PositionMap::new("\na"),
                    Global,
                )
                .unwrap(),
                indoc!(
                    "
                    (foo
                      bar)
                    "
                )
            );
        }

        #[test]
        fn format_blank_lines() {
            assert_eq!(
                format(
                    &[Expression::List(
                        "(",
                        ")",
                        vec![
                            Expression::Symbol("foo", Position::new(1, 4)),
                            Expression::Symbol("bar", Position::new(6, 9))
                        ],
                        Position::new(0, 10)
                    )],
                    &[],
                    &[],
                    &PositionMap::new("(foo\n\nbar)"),
                    Global,
                )
                .unwrap(),
                indoc!(
                    "
                    (foo

                      bar)
                    "
                )
            );
        }

        #[test]
        fn format_blank_lines_with_multi_line_expression() {
            assert_eq!(
                format(
                    &[Expression::List(
                        "(",
                        ")",
                        vec![
                            Expression::List(
                                "(",
                                ")",
                                vec![
                                    Expression::Symbol("foo", Position::new(2, 5)),
                                    Expression::Symbol("bar", Position::new(6, 9))
                                ],
                                Position::new(1, 10)
                            ),
                            Expression::Symbol("baz", Position::new(12, 15))
                        ],
                        Position::new(0, 16)
                    )],
                    &[],
                    &[],
                    &PositionMap::new("((foo\nbar)\n\nbaz)"),
                    Global,
                )
                .unwrap(),
                indoc!(
                    "
                    ((foo
                        bar)

                      baz)
                    "
                )
            );
        }

        #[test]
        fn format_first_element_on_second_line() {
            assert_eq!(
                format(
                    &[Expression::List(
                        "(",
                        ")",
                        vec![Expression::Symbol("foo", Position::new(2, 5))],
                        Position::new(0, 6)
                    )],
                    &[],
                    &[],
                    &PositionMap::new("(\nfoo)"),
                    Global,
                )
                .unwrap(),
                indoc!(
                    "
                    (
                      foo)
                    "
                )
            );
        }

        #[test]
        fn format_broken_nested_lists() {
            assert_eq!(
                format(
                    &[Expression::List(
                        "(",
                        ")",
                        vec![Expression::List(
                            "(",
                            ")",
                            vec![
                                Expression::Symbol("foo", Position::new(0, 1)),
                                Expression::Symbol("bar", Position::new(6, 7))
                            ],
                            Position::new(0, 1)
                        ),],
                        Position::new(0, 1)
                    )],
                    &[],
                    &[],
                    &PositionMap::new("((foo\nbar))"),
                    Global,
                )
                .unwrap(),
                indoc!(
                    "
                    ((foo
                        bar))
                    "
                )
            );
        }

        #[test]
        fn format_broken_nested_lists_with_offside() {
            assert_eq!(
                format(
                    &[Expression::List(
                        "(",
                        ")",
                        vec![
                            Expression::Symbol("foo", Position::new(0, 1)),
                            Expression::List(
                                "(",
                                ")",
                                vec![
                                    Expression::Symbol("bar", Position::new(0, 1)),
                                    Expression::Symbol("baz", Position::new(10, 11))
                                ],
                                Position::new(0, 1)
                            ),
                        ],
                        Position::new(0, 1)
                    )],
                    &[],
                    &[],
                    &PositionMap::new("((foo bar\nbaz))"),
                    Global,
                )
                .unwrap(),
                indoc!(
                    "
                    (foo (bar
                          baz))
                    "
                )
            );
        }
    }

    mod module {
        use super::*;
        use pretty_assertions::assert_eq;

        #[test]
        fn keep_no_blank_line() {
            assert_eq!(
                format(
                    &[
                        Expression::Symbol("foo", Position::new(0, 1)),
                        Expression::Symbol("bar", Position::new(1, 2))
                    ],
                    &[],
                    &[],
                    &PositionMap::new("\na"),
                    Global,
                )
                .unwrap(),
                indoc!(
                    "
                    foo
                    bar
                    "
                )
            );
        }

        #[test]
        fn keep_blank_line() {
            assert_eq!(
                format(
                    &[
                        Expression::Symbol("foo", Position::new(0, 1)),
                        Expression::Symbol("bar", Position::new(2, 3))
                    ],
                    &[],
                    &[],
                    &PositionMap::new("\n\na"),
                    Global,
                )
                .unwrap(),
                indoc!(
                    "
                    foo

                    bar
                    "
                )
            );
        }
    }

    mod comment {
        use super::*;
        use allocator_api2::vec;
        use pretty_assertions::assert_eq;

        #[test]
        fn format_multi_line_comment() {
            assert_eq!(
                format(
                    &[Expression::Symbol("foo", Position::new(1, 2))],
                    &[LineComment::new("bar", Position::new(0, 1)).into()],
                    &[],
                    &PositionMap::new("\na"),
                    Global,
                )
                .unwrap(),
                indoc!(
                    "
                    ;bar
                    foo
                    "
                )
            );
        }

        #[test]
        fn format_multi_line_comment_with_extra_line() {
            assert_eq!(
                format(
                    &[Expression::Symbol("foo", Position::new(2, 3))],
                    &[LineComment::new("bar", Position::new(0, 1)).into()],
                    &[],
                    &PositionMap::new("\n\na"),
                    Global,
                )
                .unwrap(),
                indoc!(
                    "
                    ;bar

                    foo
                    "
                )
            );
        }

        #[test]
        fn format_multi_line_comments() {
            assert_eq!(
                format(
                    &[Expression::Symbol("foo", Position::new(3, 4))],
                    &[
                        LineComment::new("bar", Position::new(0, 1)).into(),
                        LineComment::new("baz", Position::new(1, 2)).into()
                    ],
                    &[],
                    &PositionMap::new("\n\n\na"),
                    Global,
                )
                .unwrap(),
                indoc!(
                    "
                    ;bar
                    ;baz

                    foo
                    "
                )
            );
        }

        #[test]
        fn format_multi_line_comments_with_extra_line() {
            assert_eq!(
                format(
                    &[Expression::Symbol("foo", Position::new(4, 5))],
                    &[
                        LineComment::new("bar", Position::new(0, 1)).into(),
                        LineComment::new("baz", Position::new(2, 3)).into()
                    ],
                    &[],
                    &PositionMap::new("\n\n\n\na"),
                    Global,
                )
                .unwrap(),
                indoc!(
                    "
                    ;bar

                    ;baz

                    foo
                    "
                )
            );
        }

        #[test]
        fn format_multi_line_comment_after_expression() {
            assert_eq!(
                format(
                    &[
                        Expression::Symbol("foo", Position::new(0, 1)),
                        Expression::Symbol("baz", Position::new(2, 3))
                    ],
                    &[LineComment::new("bar", Position::new(1, 2)).into()],
                    &[],
                    &PositionMap::new("\n\n\n"),
                    Global,
                )
                .unwrap(),
                indoc!(
                    "
                    foo
                    ;bar
                    baz
                    "
                )
            );
        }

        #[test]
        fn format_multi_line_comment_after_expression_in_list() {
            assert_eq!(
                format(
                    &[Expression::List(
                        "(",
                        ")",
                        vec![
                            Expression::Symbol("foo", Position::new(0, 1)),
                            Expression::Symbol("baz", Position::new(2, 3))
                        ],
                        Position::new(0, 1)
                    )],
                    &[LineComment::new("bar", Position::new(1, 2)).into()],
                    &[],
                    &PositionMap::new("\n\n\n"),
                    Global,
                )
                .unwrap(),
                indoc!(
                    "
                    (foo
                      ;bar
                      baz)
                    "
                )
            );
        }

        #[test]
        fn format_line_comment() {
            assert_eq!(
                format(
                    &[Expression::Symbol("foo", Position::new(0, 1))],
                    &[LineComment::new("bar", Position::new(0, 1)).into()],
                    &[],
                    &PositionMap::new("\na"),
                    Global,
                )
                .unwrap(),
                indoc!(
                    "
                    foo ;bar
                    "
                )
            );
        }

        #[test]
        fn format_line_comments() {
            assert_eq!(
                format(
                    &[Expression::Symbol("foo", Position::new(0, 1))],
                    &[
                        LineComment::new("bar", Position::new(0, 1)).into(),
                        LineComment::new("baz", Position::new(0, 1)).into()
                    ],
                    &[],
                    &PositionMap::new("\na"),
                    Global,
                )
                .unwrap(),
                indoc!(
                    "
                    foo ;bar ;baz
                    "
                )
            );
        }

        #[test]
        fn format_line_comment_on_multi_line_expression() {
            assert_eq!(
                format(
                    &[Expression::List(
                        "(",
                        ")",
                        vec![Expression::Symbol("foo", Position::new(6, 9))],
                        Position::new(0, 10)
                    )],
                    &[LineComment::new("bar", Position::new(1, 5)).into()],
                    &[],
                    &PositionMap::new("(;bar\nfoo)"),
                    Global,
                )
                .unwrap(),
                indoc!(
                    "
                    ( ;bar
                      foo)
                    "
                )
            );
        }

        #[test]
        fn format_line_comment_for_last_argument_in_different_line() {
            assert_eq!(
                format(
                    &[Expression::List(
                        "(",
                        ")",
                        vec![
                            Expression::Symbol("f", Position::new(0, 1)),
                            Expression::Symbol("x", Position::new(2, 3))
                        ],
                        Position::new(0, 1)
                    )],
                    &[LineComment::new("foo", Position::new(1, 2)).into()],
                    &[],
                    &PositionMap::new("\n\n\n"),
                    Global,
                )
                .unwrap(),
                indoc!(
                    "
                    (f
                      ;foo
                      x)
                    "
                )
            );
        }

        #[test]
        fn format_remaining_multi_line_comment() {
            assert_eq!(
                format(
                    &[Expression::Symbol("foo", Position::new(0, 1))],
                    &[LineComment::new("bar", Position::new(1, 2)).into()],
                    &[],
                    &PositionMap::new("\n\n"),
                    Global,
                )
                .unwrap(),
                indoc!(
                    "
                    foo

                    ;bar
                    "
                )
            );
        }

        #[test]
        fn format_remaining_multi_line_comments() {
            assert_eq!(
                format(
                    &[Expression::Symbol("foo", Position::new(0, 1))],
                    &[
                        LineComment::new("bar", Position::new(1, 2)).into(),
                        LineComment::new("baz", Position::new(2, 3)).into()
                    ],
                    &[],
                    &PositionMap::new("\n\n\n"),
                    Global,
                )
                .unwrap(),
                indoc!(
                    "
                    foo

                    ;bar
                    ;baz
                    "
                )
            );
        }

        #[test]
        fn format_remaining_multi_line_comments_with_extra_line() {
            assert_eq!(
                format(
                    &[Expression::Symbol("foo", Position::new(0, 1))],
                    &[
                        LineComment::new("bar", Position::new(1, 2)).into(),
                        LineComment::new("baz", Position::new(3, 4)).into()
                    ],
                    &[],
                    &PositionMap::new("\n\n\n\n"),
                    Global,
                )
                .unwrap(),
                indoc!(
                    "
                    foo

                    ;bar

                    ;baz
                    "
                )
            );
        }

        #[test]
        fn format_line_comment_before_closing_parenthesis() {
            assert_eq!(
                format(
                    &[Expression::List(
                        "(",
                        ")",
                        vec![
                            Expression::Symbol("foo", Position::new(0, 1)),
                            Expression::Symbol("bar", Position::new(1, 2))
                        ],
                        Position::new(0, 5)
                    )],
                    &[
                        LineComment::new("baz", Position::new(2, 3)).into(),
                        LineComment::new("qux", Position::new(3, 4)).into()
                    ],
                    &[],
                    &PositionMap::new("\n\n\n\n\n"),
                    Global,
                )
                .unwrap(),
                indoc!(
                    "
                    (foo
                      bar
                      ;baz
                      ;qux
                      )
                    "
                )
            );
        }

        #[test]
        fn format_line_and_inline_comments_before_closing_parenthesis() {
            assert_eq!(
                format(
                    &[Expression::List(
                        "(",
                        ")",
                        vec![
                            Expression::Symbol("foo", Position::new(0, 1)),
                            Expression::Symbol("bar", Position::new(1, 2))
                        ],
                        Position::new(0, 6)
                    )],
                    &[
                        LineComment::new("baz", Position::new(2, 3)).into(),
                        LineComment::new("qux", Position::new(3, 4)).into(),
                        BlockComment::new("quux", Position::new(4, 5)).into(),
                        BlockComment::new("blah", Position::new(4, 5)).into(),
                    ],
                    &[],
                    &PositionMap::new("\n\n\n\n)\n"),
                    Global,
                )
                .unwrap(),
                indoc!(
                    "
                    (foo
                      bar
                      ;baz
                      ;qux
                      #|quux|# #|blah|#)
                    "
                )
            );
        }

        #[test]
        fn format_line_and_inline_comments_before_closing_parenthesis_in_nested_expression() {
            assert_eq!(
                format(
                    &[Expression::List(
                        "(",
                        ")",
                        vec![
                            Expression::Symbol("foo", Position::new(0, 1)),
                            Expression::List(
                                "(",
                                ")",
                                vec![
                                    Expression::Symbol("foo", Position::new(0, 1)),
                                    Expression::Symbol("bar", Position::new(1, 2))
                                ],
                                Position::new(0, 6)
                            )
                        ],
                        Position::new(0, 6)
                    )],
                    &[
                        LineComment::new("baz", Position::new(2, 3)).into(),
                        LineComment::new("qux", Position::new(3, 4)).into(),
                        BlockComment::new("quux", Position::new(4, 5)).into(),
                        BlockComment::new("blah", Position::new(4, 5)).into(),
                    ],
                    &[],
                    &PositionMap::new("\n\n\n\n)\n"),
                    Global,
                )
                .unwrap(),
                indoc!(
                    "
                    (foo (foo
                          bar
                          ;baz
                          ;qux
                          #|quux|# #|blah|#))
                    "
                )
            );
        }

        mod block {
            use super::*;

            mod inline {
                use super::*;
                use allocator_api2::vec;
                use pretty_assertions::assert_eq;

                #[test]
                fn format_in_front() {
                    assert_eq!(
                        format(
                            &[Expression::Symbol("bar", Position::new(8, 11))],
                            &[BlockComment::new("foo", Position::new(0, 8)).into(),],
                            &[],
                            &PositionMap::new("#|foo|#bar"),
                            Global,
                        )
                        .unwrap(),
                        indoc!(
                            "
                            #|foo|# bar
                            "
                        )
                    );
                }

                #[test]
                fn format_after_first_expression() {
                    assert_eq!(
                        format(
                            &[Expression::List(
                                "(",
                                ")",
                                vec![
                                    Expression::Symbol("foo", Position::new(1, 4)),
                                    Expression::Symbol("baz", Position::new(11, 14))
                                ],
                                Position::new(0, 15)
                            ),],
                            &[BlockComment::new("bar", Position::new(4, 11)).into(),],
                            &[],
                            &PositionMap::new("(foo#|bar|#baz)"),
                            Global,
                        )
                        .unwrap(),
                        indoc!(
                            "
                            (foo #|bar|# baz)
                            "
                        )
                    );
                }

                #[test]
                fn format_after_second_expression() {
                    assert_eq!(
                        format(
                            &[Expression::List(
                                "(",
                                ")",
                                vec![
                                    Expression::Symbol("foo", Position::new(1, 4)),
                                    Expression::Symbol("bar", Position::new(5, 8)),
                                    Expression::Symbol("qux", Position::new(15, 18)),
                                ],
                                Position::new(0, 19)
                            ),],
                            &[BlockComment::new("baz", Position::new(8, 15)).into(),],
                            &[],
                            &PositionMap::new("(foo bar#|baz|#qux)"),
                            Global,
                        )
                        .unwrap(),
                        indoc!(
                            "
                            (foo bar #|baz|# qux)
                            "
                        )
                    );
                }

                #[test]
                fn format_after_third_expression() {
                    assert_eq!(
                        format(
                            &[Expression::List(
                                "(",
                                ")",
                                vec![
                                    Expression::Symbol("foo", Position::new(1, 4)),
                                    Expression::Symbol("bar", Position::new(5, 8)),
                                    Expression::Symbol("qux", Position::new(9, 12)),
                                ],
                                Position::new(0, 20)
                            ),],
                            &[BlockComment::new("baz", Position::new(12, 19)).into(),],
                            &[],
                            &PositionMap::new("(foo bar qux#|baz|#)"),
                            Global,
                        )
                        .unwrap(),
                        indoc!(
                            "
                            (foo bar qux #|baz|#)
                            "
                        )
                    );
                }

                #[test]
                fn format_in_empty_list() {
                    assert_eq!(
                        format(
                            &[Expression::List("(", ")", vec![], Position::new(0, 9))],
                            &[BlockComment::new("foo", Position::new(1, 8)).into(),],
                            &[],
                            &PositionMap::new("(#|foo|#)"),
                            Global,
                        )
                        .unwrap(),
                        indoc!(
                            "
                            (#|foo|#)
                            "
                        )
                    );
                }

                #[test]
                fn format_many() {
                    assert_eq!(
                        format(
                            &[Expression::List("(", ")", vec![], Position::new(0, 2))],
                            &[
                                BlockComment::new("foo", Position::new(0, 1)).into(),
                                BlockComment::new("bar", Position::new(0, 1)).into(),
                                BlockComment::new("baz", Position::new(0, 1)).into(),
                            ],
                            &[],
                            &PositionMap::new("a\n"),
                            Global,
                        )
                        .unwrap(),
                        indoc!(
                            "
                            (#|foo|# #|bar|# #|baz|#)
                            "
                        )
                    );
                }
            }

            mod multi_line {
                use super::*;
                use pretty_assertions::assert_eq;

                #[test]
                fn format_with_no_blank_line() {
                    assert_eq!(
                        format(
                            &[
                                Expression::Symbol("foo", Position::new(0, 1)),
                                Expression::Symbol("bar", Position::new(12, 13))
                            ],
                            &[BlockComment::new("foo", Position::new(4, 5)).into(),],
                            &[],
                            &PositionMap::new("foo\n#|foo|#\nbar"),
                            Global,
                        )
                        .unwrap(),
                        indoc!(
                            "
                            foo
                            #|
                            foo
                            |#
                            bar
                            "
                        )
                    );
                }

                #[test]
                fn format_with_blank_lines() {
                    assert_eq!(
                        format(
                            &[
                                Expression::Symbol("foo", Position::new(0, 1)),
                                Expression::Symbol("bar", Position::new(14, 15))
                            ],
                            &[BlockComment::new("foo", Position::new(5, 6)).into(),],
                            &[],
                            &PositionMap::new("foo\n\n#|foo|#\n\nbar"),
                            Global,
                        )
                        .unwrap(),
                        indoc!(
                            "
                            foo

                            #|
                            foo
                            |#

                            bar
                            "
                        )
                    );
                }

                #[test]
                fn format_with_extra_blank_lines() {
                    assert_eq!(
                        format(
                            &[
                                Expression::Symbol("foo", Position::new(0, 1)),
                                Expression::Symbol("bar", Position::new(16, 17))
                            ],
                            &[BlockComment::new("foo", Position::new(6, 7)).into(),],
                            &[],
                            &PositionMap::new("foo\n\n\n#|foo|#\n\n\nbar"),
                            Global,
                        )
                        .unwrap(),
                        indoc!(
                            "
                            foo

                            #|
                            foo
                            |#

                            bar
                            "
                        )
                    );
                }
            }
        }
    }

    mod hash_directive {
        use super::*;
        use pretty_assertions::assert_eq;

        #[test]
        fn format_hash_directive() {
            assert_eq!(
                format(
                    &[],
                    &[],
                    &[HashDirective::new("foo", Position::new(0, 0))],
                    &PositionMap::new("\n"),
                    Global,
                )
                .unwrap(),
                indoc!(
                    "
                    #foo
                    "
                )
            );
        }

        #[test]
        fn format_hash_directives() {
            assert_eq!(
                format(
                    &[],
                    &[],
                    &[
                        HashDirective::new("foo", Position::new(0, 0)),
                        HashDirective::new("bar", Position::new(2, 2))
                    ],
                    &PositionMap::new("\n\n\n"),
                    Global,
                )
                .unwrap(),
                indoc!(
                    "
                    #foo
                    #bar
                    "
                )
            );
        }

        #[test]
        fn format_hash_directive_with_expression() {
            assert_eq!(
                format(
                    &[Expression::Symbol("bar", Position::new(0, 0))],
                    &[],
                    &[HashDirective::new("foo", Position::new(0, 0))],
                    &PositionMap::new("\n"),
                    Global,
                )
                .unwrap(),
                indoc!(
                    "
                    #foo

                    bar
                    "
                )
            );
        }
    }

    mod data {
        use super::*;
        use allocator_api2::vec;
        use pretty_assertions::assert_eq;

        #[test]
        fn format_list() {
            assert_eq!(
                format(
                    &[Expression::Quote(
                        "'",
                        Expression::List(
                            "(",
                            ")",
                            vec![
                                Expression::Symbol("foo", Position::new(0, 1)),
                                Expression::Symbol("bar", Position::new(0, 1))
                            ],
                            Position::new(0, 1)
                        )
                        .into(),
                        Position::new(0, 1)
                    )],
                    &[],
                    &[],
                    &PositionMap::new("(foo bar)"),
                    Global,
                )
                .unwrap(),
                "'(foo bar)\n"
            );
        }

        #[test]
        fn format_broken_list() {
            assert_eq!(
                format(
                    &[Expression::Quote(
                        "'",
                        Expression::List(
                            "(",
                            ")",
                            vec![
                                Expression::Symbol("foo", Position::new(0, 1)),
                                Expression::Symbol("bar", Position::new(6, 7))
                            ],
                            Position::new(0, 1)
                        )
                        .into(),
                        Position::new(0, 1)
                    )],
                    &[],
                    &[],
                    &PositionMap::new("'(foo\nbar)"),
                    Global,
                )
                .unwrap(),
                indoc!(
                    "
                    '(foo
                      bar)
                    "
                )
            );
        }

        #[test]
        fn format_broken_list_with_empty_line() {
            assert_eq!(
                format(
                    &[Expression::Quote(
                        "'",
                        Expression::List(
                            "(",
                            ")",
                            vec![
                                Expression::Symbol("foo", Position::new(3, 4)),
                                Expression::Symbol("bar", Position::new(3, 4))
                            ],
                            Position::new(0, 1)
                        )
                        .into(),
                        Position::new(0, 1)
                    )],
                    &[],
                    &[],
                    &PositionMap::new("'(\nfoo\nbar)"),
                    Global,
                )
                .unwrap(),
                indoc!(
                    "
                    '(
                      foo
                      bar)
                    "
                )
            );
        }

        #[test]
        fn format_line_comment_on_multi_line_expression() {
            assert_eq!(
                format(
                    &[Expression::Quote(
                        "'",
                        Expression::List(
                            "(",
                            ")",
                            vec![Expression::Symbol("foo", Position::new(7, 10))],
                            Position::new(1, 11)
                        )
                        .into(),
                        Position::new(0, 11)
                    )],
                    &[LineComment::new("bar", Position::new(2, 6)).into()],
                    &[],
                    &PositionMap::new("'(;bar\nfoo)"),
                    Global,
                )
                .unwrap(),
                indoc!(
                    "
                    '( ;bar
                      foo)
                    "
                )
            );
        }

        #[test]
        fn format_first_element_on_second_line() {
            assert_eq!(
                format(
                    &[Expression::Quote(
                        "'",
                        Expression::List(
                            "(",
                            ")",
                            vec![Expression::Symbol("foo", Position::new(3, 6))],
                            Position::new(1, 7)
                        )
                        .into(),
                        Position::new(0, 7)
                    )],
                    &[],
                    &[],
                    &PositionMap::new("'(\nfoo)"),
                    Global,
                )
                .unwrap(),
                indoc!(
                    "
                    '(
                      foo)
                    "
                )
            );
        }

        mod nested {
            use super::*;
            use allocator_api2::vec;
            use pretty_assertions::assert_eq;

            #[test]
            fn format_nested_lists() {
                assert_eq!(
                    format(
                        &[Expression::Quote(
                            "'",
                            Expression::List(
                                "(",
                                ")",
                                vec![Expression::List(
                                    "(",
                                    ")",
                                    vec![
                                        Expression::Symbol("foo", Position::new(0, 1)),
                                        Expression::Symbol("bar", Position::new(0, 1))
                                    ],
                                    Position::new(0, 1)
                                ),],
                                Position::new(0, 1)
                            )
                            .into(),
                            Position::new(0, 1)
                        )],
                        &[],
                        &[],
                        &PositionMap::new("'((foo bar))"),
                        Global,
                    )
                    .unwrap(),
                    indoc!(
                        "
                        '((foo bar))
                        "
                    )
                );
            }

            #[test]
            fn format_broken_nested_lists() {
                assert_eq!(
                    format(
                        &[Expression::Quote(
                            "'",
                            Expression::List(
                                "(",
                                ")",
                                vec![Expression::List(
                                    "(",
                                    ")",
                                    vec![
                                        Expression::Symbol("foo", Position::new(0, 1)),
                                        Expression::Symbol("bar", Position::new(7, 8))
                                    ],
                                    Position::new(0, 1)
                                ),],
                                Position::new(0, 1)
                            )
                            .into(),
                            Position::new(0, 1)
                        )],
                        &[],
                        &[],
                        &PositionMap::new("'((foo\nbar))"),
                        Global,
                    )
                    .unwrap(),
                    indoc!(
                        "
                        '((foo
                           bar))
                        "
                    )
                );
            }

            #[test]
            fn format_nested_unquote() {
                assert_eq!(
                    format(
                        &[Expression::Quote(
                            ",",
                            Expression::List(
                                "(",
                                ")",
                                vec![Expression::List(
                                    "(",
                                    ")",
                                    vec![
                                        Expression::Symbol("foo", Position::new(0, 1)),
                                        Expression::Symbol("bar", Position::new(7, 8))
                                    ],
                                    Position::new(0, 1)
                                ),],
                                Position::new(0, 1)
                            )
                            .into(),
                            Position::new(0, 1)
                        )],
                        &[],
                        &[],
                        &PositionMap::new(",((foo\nbar))"),
                        Global,
                    )
                    .unwrap(),
                    indoc!(
                        "
                        ,((foo
                            bar))
                        "
                    )
                );
            }

            #[test]
            fn format_splicing_unquote() {
                assert_eq!(
                    format(
                        &[Expression::Quote(
                            ",",
                            Expression::Quote(
                                "@",
                                Expression::List(
                                    "(",
                                    ")",
                                    vec![
                                        Expression::Symbol("foo", Position::new(3, 6)),
                                        Expression::List(
                                            "(",
                                            ")",
                                            vec![
                                                Expression::Symbol("bar", Position::new(8, 11)),
                                                Expression::Symbol("baz", Position::new(12, 15)),
                                            ],
                                            Position::new(7, 16)
                                        ),
                                    ],
                                    Position::new(2, 17)
                                )
                                .into(),
                                Position::new(1, 17)
                            )
                            .into(),
                            Position::new(0, 17)
                        )],
                        &[],
                        &[],
                        &PositionMap::new(",@(foo\n(bar\nbaz))"),
                        Global,
                    )
                    .unwrap(),
                    indoc!(
                        "
                        ,@(foo
                           (bar
                             baz))
                        "
                    )
                );
            }
        }
    }
}
