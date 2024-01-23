use crate::{
    ast::{Comment, Expression, HashDirective},
    context::Context,
    position::Position,
    position_map::PositionMap,
};
use core::fmt;
use mfmt::{empty, line, sequence, utility::is_empty, Builder, Document, FormatOptions};
use std::alloc::Allocator;

const COMMENT_PREFIX: &str = ";";

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
    compile_line_comment(context, expression.position(), |context| match expression {
        Expression::List(left, right, expressions, position) => {
            compile_list(context, expressions, position, left, right, data)
        }
        Expression::Quote(sign, expression, _) => {
            let builder = context.builder().clone();

            builder.sequence([
                (*sign).into(),
                compile_expression(context, expression, *sign != ","),
            ])
        }
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
    let line_index = get_line_index(context, position.start());

    let index = expressions
        .iter()
        .position(|expression| get_line_index(context, expression.position().start()) != line_index)
        .unwrap_or(expressions.len());
    let first = &expressions[..index];
    let last = &expressions[index..];

    let builder = context.builder().clone();

    builder.sequence([
        compile_line_comment(context, position, |_| left.into()),
        builder.indent(
            builder.offside(
                builder.sequence(
                    [builder.flatten(compile_expressions(context, first, data))]
                        .into_iter()
                        .chain(match (first.last(), last.first()) {
                            (Some(first), Some(last)) if has_extra_line(context, first, last) => {
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
                        }),
                ),
                !data,
            ),
        ),
        right.into(),
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
            documents.extend(if has_extra_line(context, last_expression, expression) {
                Some(line())
            } else {
                None
            });
        }

        documents.push(compile_expression(context, expression, data));

        last_expression = Some(expression);
    }

    sequence(documents.leak())
}

fn compile_line_comment<'a, A: Allocator + Clone + 'a>(
    context: &mut Context<'a, A>,
    position: &Position,
    document: impl Fn(&mut Context<'a, A>) -> Document<'a>,
) -> Document<'a> {
    let block_comment = compile_block_comment(context, position);
    let document = document(context);
    let suffix_comment = compile_suffix_comment(context, position);

    context
        .builder()
        .sequence([block_comment, document, suffix_comment])
}

fn compile_suffix_comment<'a, A: Allocator + Clone + 'a>(
    context: &mut Context<A>,
    position: &Position,
) -> Document<'a> {
    let builder = context.builder().clone();

    builder.sequence(
        context
            .drain_current_comment(get_line_index(context, position.start()))
            .map(|comment| {
                builder.line_suffixes([" ", COMMENT_PREFIX, comment.value().trim_end()])
            }),
    )
}

fn compile_block_comment<'a, A: Allocator + Clone + 'a>(
    context: &mut Context<'a, A>,
    position: &Position,
) -> Document<'a> {
    let builder = context.builder().clone();
    let comments = builder
        .allocate_slice(context.drain_comments_before(get_line_index(context, position.start())));

    compile_all_comments(
        context,
        comments,
        Some(get_line_index(context, position.start())),
    )
}

fn compile_remaining_block_comment<'a, A: Allocator + Clone + 'a>(
    context: &mut Context<'a, A>,
) -> Document<'a> {
    let builder = context.builder().clone();
    let comments = builder.allocate_slice(context.drain_comments_before(usize::MAX));

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
                    .map(|comment| get_line_index(context, comment.position().start()))
                    .chain([last_line_index.unwrap_or(0)]),
            )
            .map(|(comment, next_line_index)| {
                context.builder().sequence([
                    COMMENT_PREFIX.into(),
                    comment.value().trim_end().into(),
                    context.builder().r#break(line()),
                    if get_line_index(context, comment.position().end() - 1) + 1 < next_line_index {
                        line()
                    } else {
                        empty()
                    },
                ])
            }),
    )
}

fn has_extra_line<A: Allocator + Clone>(
    context: &Context<A>,
    last_expression: &Expression<A>,
    expression: &Expression<A>,
) -> bool {
    let line_index = get_line_index(context, expression.position().start());

    context
        .peek_comments_before(line_index)
        .next()
        .map(|comment| get_line_index(context, comment.position().start()))
        .unwrap_or(line_index)
        .saturating_sub(get_line_index(
            context,
            last_expression.position().end() - 1,
        ))
        > 1
}

fn get_line_index<A: Allocator + Clone>(context: &Context<A>, offset: usize) -> usize {
    context
        .position_map()
        .line_index(offset)
        .expect("valid offset")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{position::Position, position_map::PositionMap};
    use indoc::indoc;
    use pretty_assertions::assert_eq;
    use std::alloc::Global;

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
        fn keep_no_blank_line() {
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
        fn keep_blank_line() {
            assert_eq!(
                format(
                    &[Expression::List(
                        "(",
                        ")",
                        vec![
                            Expression::Symbol("foo", Position::new(0, 1)),
                            Expression::Symbol("bar", Position::new(2, 3))
                        ],
                        Position::new(0, 1)
                    )],
                    &[],
                    &[],
                    &PositionMap::new("\n\na"),
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
        fn keep_blank_line_with_multi_line_expression() {
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
                                    Expression::Symbol("foo", Position::new(0, 1)),
                                    Expression::Symbol("bar", Position::new(1, 2))
                                ],
                                Position::new(0, 1)
                            ),
                            Expression::Symbol("baz", Position::new(3, 4))
                        ],
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
                        vec![Expression::Symbol("foo", Position::new(1, 1))],
                        Position::new(0, 0)
                    )],
                    &[],
                    &[],
                    &PositionMap::new("\n\n"),
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
        use pretty_assertions::assert_eq;

        #[test]
        fn format_block_comment() {
            assert_eq!(
                format(
                    &[Expression::Symbol("foo", Position::new(1, 2))],
                    &[Comment::new("bar", Position::new(0, 1))],
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
        fn format_block_comment_with_extra_line() {
            assert_eq!(
                format(
                    &[Expression::Symbol("foo", Position::new(2, 3))],
                    &[Comment::new("bar", Position::new(0, 1))],
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
        fn format_block_comments() {
            assert_eq!(
                format(
                    &[Expression::Symbol("foo", Position::new(3, 4))],
                    &[
                        Comment::new("bar", Position::new(0, 1)),
                        Comment::new("baz", Position::new(1, 2))
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
        fn format_block_comments_with_extra_line() {
            assert_eq!(
                format(
                    &[Expression::Symbol("foo", Position::new(4, 5))],
                    &[
                        Comment::new("bar", Position::new(0, 1)),
                        Comment::new("baz", Position::new(2, 3))
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
        fn format_block_comment_after_expression() {
            assert_eq!(
                format(
                    &[
                        Expression::Symbol("foo", Position::new(0, 1)),
                        Expression::Symbol("baz", Position::new(2, 3))
                    ],
                    &[Comment::new("bar", Position::new(1, 2))],
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
        fn format_block_comment_after_expression_in_list() {
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
                    &[Comment::new("bar", Position::new(1, 2))],
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
                    &[Comment::new("bar", Position::new(0, 1))],
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
                        Comment::new("bar", Position::new(0, 1)),
                        Comment::new("baz", Position::new(0, 1))
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
                        vec![Expression::Symbol("foo", Position::new(1, 2))],
                        Position::new(0, 1)
                    )],
                    &[Comment::new("bar", Position::new(0, 1))],
                    &[],
                    &PositionMap::new("\n\n"),
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
                    &[Comment::new("foo", Position::new(1, 2))],
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
        fn format_remaining_block_comment() {
            assert_eq!(
                format(
                    &[Expression::Symbol("foo", Position::new(0, 1))],
                    &[Comment::new("bar", Position::new(1, 2))],
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
        fn format_remaining_block_comments() {
            assert_eq!(
                format(
                    &[Expression::Symbol("foo", Position::new(0, 1))],
                    &[
                        Comment::new("bar", Position::new(1, 2)),
                        Comment::new("baz", Position::new(2, 3))
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
        fn format_remaining_block_comments_with_extra_line() {
            assert_eq!(
                format(
                    &[Expression::Symbol("foo", Position::new(0, 1))],
                    &[
                        Comment::new("bar", Position::new(1, 2)),
                        Comment::new("baz", Position::new(3, 4))
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

        mod nested {
            use super::*;
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
        }
    }
}
