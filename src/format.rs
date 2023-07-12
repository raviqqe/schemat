use crate::{
    ast::{Comment, Expression},
    context::Context,
    position::Position,
    position_map::PositionMap,
};
use mfmt::{empty, flatten, indent, line, line_suffix, r#break, sequence, Document};

pub fn format(module: &[Expression], comments: &[Comment], position_map: &PositionMap) -> String {
    mfmt::format(&compile_module(
        &mut Context::new(comments, position_map),
        module,
    ))
}

fn compile_module(context: &mut Context, module: &[Expression]) -> Document {
    sequence([compile_expressions(context, module), line(), {
        let comment = compile_remaining_block_comment(context);

        if mfmt::utility::count_lines(&comment) > 0 {
            sequence([line(), comment])
        } else {
            empty()
        }
    }])
}

fn compile_expression(context: &mut Context, expression: &Expression) -> Document {
    compile_line_comment(context, expression.position(), |context| match expression {
        Expression::List(expressions, position) => {
            let line_index = get_line_index(context, position.start());
            let (first, last) = expressions.iter().partition::<Vec<_>, _>(|expression| {
                get_line_index(context, expression.position().start()) == line_index
            });
            let extra_line = match (first.last(), last.first()) {
                (Some(first), Some(last)) if has_extra_line(context, first, last) => Some(line()),
                _ => None,
            };

            flatten(sequence(
                [compile_line_comment(context, expression.position(), |_| {
                    "(".into()
                })]
                .into_iter()
                .chain([indent(compile_expressions(context, first))])
                .chain(if last.is_empty() {
                    None
                } else {
                    Some(r#break(indent(sequence(
                        extra_line
                            .into_iter()
                            .chain([line(), compile_expressions(context, last)]),
                    ))))
                })
                .chain([")".into()])
                .collect::<Vec<_>>(),
            ))
        }
        Expression::String(string, _) => sequence(["\"", string, "\""]),
        Expression::Symbol(name, _) => (*name).into(),
        Expression::Quote(expression, _) => {
            sequence(["'".into(), compile_expression(context, expression)])
        }
    })
}

fn compile_expressions<'a>(
    context: &mut Context,
    expressions: impl IntoIterator<Item = &'a Expression<'a>>,
) -> Document {
    let mut documents = vec![];
    let mut last_expression = None::<&Expression>;

    for expression in expressions {
        if let Some(last_expression) = last_expression {
            documents.push(line());
            documents.extend(if has_extra_line(context, last_expression, expression) {
                Some(line())
            } else {
                None
            });
        }

        documents.push(compile_expression(context, expression));

        last_expression = Some(expression);
    }

    sequence(documents)
}

fn compile_line_comment(
    context: &mut Context,
    position: &Position,
    document: impl Fn(&mut Context) -> Document,
) -> Document {
    sequence([
        compile_block_comment(context, position),
        document(context),
        compile_suffix_comment(context, position),
    ])
}

fn compile_suffix_comment(context: &mut Context, position: &Position) -> Document {
    sequence(
        context
            .drain_current_comment(get_line_index(context, position.start()))
            .map(|comment| line_suffix(" ;".to_owned() + comment.value().trim_end())),
    )
}

fn compile_block_comment(context: &mut Context, position: &Position) -> Document {
    let comments = context
        .drain_comments_before(get_line_index(context, position.start()))
        .collect::<Vec<_>>();

    compile_all_comments(
        context,
        &comments,
        Some(get_line_index(context, position.start())),
    )
}

fn compile_remaining_block_comment(context: &mut Context) -> Document {
    let comments = context
        .drain_comments_before(usize::MAX)
        .collect::<Vec<_>>();

    compile_all_comments(context, &comments, None)
}

fn compile_all_comments(
    context: &Context,
    comments: &[&Comment],
    last_line_index: Option<usize>,
) -> Document {
    sequence(
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
                sequence([
                    ";".into(),
                    comment.value().trim_end().into(),
                    r#break(line()),
                    if get_line_index(context, comment.position().end() - 1) + 1 < next_line_index {
                        line()
                    } else {
                        empty()
                    },
                ])
            }),
    )
}

fn has_extra_line(
    context: &Context,
    last_expression: &Expression,
    expression: &Expression,
) -> bool {
    get_line_index(context, expression.position().start()).saturating_sub(get_line_index(
        context,
        last_expression.position().end() - 1,
    )) > 1
}

fn get_line_index(context: &Context, offset: usize) -> usize {
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

    #[test]
    fn format_list() {
        assert_eq!(
            format(
                &[Expression::List(
                    vec![
                        Expression::Symbol("foo", Position::new(0, 2)),
                        Expression::Symbol("bar", Position::new(0, 2))
                    ],
                    Position::new(0, 2)
                )],
                &[],
                &PositionMap::new("(foo bar)"),
            ),
            "(foo bar)\n"
        );
    }

    #[test]
    fn format_list_with_split_lines() {
        assert_eq!(
            format(
                &[Expression::List(
                    vec![
                        Expression::Symbol("foo", Position::new(1, 4)),
                        Expression::Symbol("bar", Position::new(5, 8))
                    ],
                    Position::new(0, 9)
                )],
                &[],
                &PositionMap::new("(foo\nbar)"),
            ),
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
                    vec![
                        Expression::Symbol("foo", Position::new(0, 0)),
                        Expression::Symbol("bar", Position::new(0, 0)),
                        Expression::Symbol("baz", Position::new(2, 0)),
                        Expression::Symbol("qux", Position::new(2, 0)),
                    ],
                    Position::new(0, 0)
                )],
                &[],
                &PositionMap::new("a\nb"),
            ),
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
                    Expression::Symbol("foo", Position::new(0, 3)).into(),
                    Position::new(0, 3)
                )],
                &[],
                &PositionMap::new("'foo"),
            ),
            "'foo\n"
        );
    }

    #[test]
    fn format_string() {
        assert_eq!(
            format(
                &[Expression::String("foo", Position::new(0, 3))],
                &[],
                &PositionMap::new("\"foo\""),
            ),
            "\"foo\"\n"
        );
    }

    #[test]
    fn format_symbol() {
        assert_eq!(
            format(
                &[Expression::Symbol("foo", Position::new(0, 3))],
                &[],
                &PositionMap::new("foo"),
            ),
            "foo\n"
        );
    }

    mod list {
        use super::*;

        #[test]
        fn double_indent_of_nested_list() {
            assert_eq!(
                format(
                    &[Expression::List(
                        vec![Expression::List(
                            vec![
                                Expression::Symbol("foo", Position::new(0, 0)),
                                Expression::Symbol("bar", Position::new(1, 1))
                            ],
                            Position::new(0, 1)
                        )],
                        Position::new(0, 0)
                    )],
                    &[],
                    &PositionMap::new("\n\n\na"),
                ),
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
                        vec![
                            Expression::Symbol("foo", Position::new(0, 0)),
                            Expression::Symbol("bar", Position::new(1, 1))
                        ],
                        Position::new(0, 0)
                    )],
                    &[],
                    &PositionMap::new("\na"),
                ),
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
                        vec![
                            Expression::Symbol("foo", Position::new(0, 0)),
                            Expression::Symbol("bar", Position::new(2, 2))
                        ],
                        Position::new(0, 0)
                    )],
                    &[],
                    &PositionMap::new("\n\na"),
                ),
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
                        vec![
                            Expression::List(
                                vec![
                                    Expression::Symbol("foo", Position::new(0, 0)),
                                    Expression::Symbol("bar", Position::new(1, 1))
                                ],
                                Position::new(0, 1)
                            ),
                            Expression::Symbol("baz", Position::new(3, 3))
                        ],
                        Position::new(0, 0)
                    )],
                    &[],
                    &PositionMap::new("\n\n\na"),
                ),
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
                        vec![Expression::Symbol("foo", Position::new(1, 1))],
                        Position::new(0, 0)
                    )],
                    &[],
                    &PositionMap::new("\n\n"),
                ),
                indoc!(
                    "
                    (
                      foo)
                    "
                )
            );
        }
    }

    mod module {
        use super::*;

        #[test]
        fn keep_no_blank_line() {
            assert_eq!(
                format(
                    &[
                        Expression::Symbol("foo", Position::new(0, 0)),
                        Expression::Symbol("bar", Position::new(1, 1))
                    ],
                    &[],
                    &PositionMap::new("\na"),
                ),
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
                        Expression::Symbol("foo", Position::new(0, 0)),
                        Expression::Symbol("bar", Position::new(2, 2))
                    ],
                    &[],
                    &PositionMap::new("\n\na"),
                ),
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
                    &[Expression::Symbol("foo", Position::new(1, 1))],
                    &[Comment::new("bar", Position::new(0, 0))],
                    &PositionMap::new("\na"),
                ),
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
                    &[Expression::Symbol("foo", Position::new(2, 2))],
                    &[Comment::new("bar", Position::new(0, 0))],
                    &PositionMap::new("\n\na"),
                ),
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
                    &[Expression::Symbol("foo", Position::new(3, 3))],
                    &[
                        Comment::new("bar", Position::new(0, 0)),
                        Comment::new("baz", Position::new(1, 1))
                    ],
                    &PositionMap::new("\n\n\na"),
                ),
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
                    &[Expression::Symbol("foo", Position::new(4, 4))],
                    &[
                        Comment::new("bar", Position::new(0, 0)),
                        Comment::new("baz", Position::new(2, 2))
                    ],
                    &PositionMap::new("\n\n\n\na"),
                ),
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
        fn format_line_comment() {
            assert_eq!(
                format(
                    &[Expression::Symbol("foo", Position::new(0, 0))],
                    &[Comment::new("bar", Position::new(0, 0))],
                    &PositionMap::new("\na"),
                ),
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
                    &[Expression::Symbol("foo", Position::new(0, 0))],
                    &[
                        Comment::new("bar", Position::new(0, 0)),
                        Comment::new("baz", Position::new(0, 0))
                    ],
                    &PositionMap::new("\na"),
                ),
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
                        vec![Expression::Symbol("foo", Position::new(1, 1))],
                        Position::new(0, 0)
                    )],
                    &[Comment::new("bar", Position::new(0, 0))],
                    &PositionMap::new("\n\n"),
                ),
                indoc!(
                    "
                    ( ;bar
                      foo)
                    "
                )
            );
        }

        #[test]
        fn format_remaining_block_comment() {
            assert_eq!(
                format(
                    &[Expression::Symbol("foo", Position::new(0, 0))],
                    &[Comment::new("bar", Position::new(1, 1))],
                    &PositionMap::new("\n\n"),
                ),
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
                    &[Expression::Symbol("foo", Position::new(0, 0))],
                    &[
                        Comment::new("bar", Position::new(1, 1)),
                        Comment::new("baz", Position::new(2, 2))
                    ],
                    &PositionMap::new("\n\n\n"),
                ),
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
                    &[Expression::Symbol("foo", Position::new(0, 0))],
                    &[
                        Comment::new("bar", Position::new(1, 1)),
                        Comment::new("baz", Position::new(3, 3))
                    ],
                    &PositionMap::new("\n\n\n\n"),
                ),
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
}
