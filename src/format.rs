use crate::{
    ast::{Comment, Expression, HashDirective},
    context::Context,
    position::Position,
    position_map::PositionMap,
};
use mfmt::{
    empty, flatten, indent, line, line_suffix, r#break, sequence,
    utility::{count_lines, is_empty},
    Document,
};
use std::alloc::Allocator;

const COMMENT_PREFIX: &str = ";";

pub fn format<A: Allocator + Clone>(
    module: &[Expression<A>],
    comments: &[Comment],
    hash_directives: &[HashDirective],
    position_map: &PositionMap,
    allocator: A,
) -> String {
    mfmt::format(&compile_module(
        &mut Context::new(comments, position_map, allocator),
        module,
        hash_directives,
    ))
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
            sequence(allocate(
                context.allocator(),
                [sequence(allocate_vec(
                    context.allocator(),
                    hash_directives
                        .iter()
                        .map(|directive| compile_hash_directive(context, directive)),
                ))],
            ))
        },
        {
            let expressions = compile_expressions(context, module);

            if is_empty(&expressions) {
                empty()
            } else {
                sequence(allocate(context.allocator(), [expressions, line()]))
            }
        },
        compile_remaining_block_comment(context),
    ]
    .into_iter()
    .fold(empty(), |all, document| {
        if count_lines(&document) == 0 {
            all
        } else {
            sequence(allocate(
                context.allocator(),
                [
                    if count_lines(&all) == 0 {
                        empty()
                    } else {
                        sequence(allocate(context.allocator(), [all, line()]))
                    },
                    document,
                ],
            ))
        }
    })
}

fn compile_hash_directive<'a, A: Allocator + Clone + 'a>(
    context: &Context<A>,
    hash_directive: &HashDirective,
) -> Document<'a> {
    sequence(allocate(
        context.allocator(),
        [
            allocate(context.allocator(), "#".to_owned() + hash_directive.value())
                .as_str()
                .into(),
            line(),
        ],
    ))
}

fn compile_expression<'a, A: Allocator + Clone + 'a>(
    context: &mut Context<'a, A>,
    expression: &'a Expression<'a, A>,
) -> Document<'a> {
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
            let comment = compile_line_comment(context, expression.position(), |_| "(".into());

            sequence(allocate_vec(
                context.allocator(),
                [comment]
                    .into_iter()
                    .chain([flatten(allocate(
                        context.allocator(),
                        indent(allocate(
                            context.allocator(),
                            compile_expressions(context, first),
                        )),
                    ))])
                    .chain(if last.is_empty() {
                        None
                    } else {
                        Some(r#break(allocate(
                            context.allocator(),
                            indent(allocate(
                                context.allocator(),
                                sequence(allocate_vec(
                                    context.allocator(),
                                    extra_line
                                        .into_iter()
                                        .chain([line(), compile_expressions(context, last)]),
                                )),
                            )),
                        )))
                    })
                    .chain([")".into()]),
            ))
        }
        Expression::String(string, _) => sequence(allocate(
            context.allocator(),
            ["\"".into(), (*string).into(), "\"".into()],
        )),
        Expression::Symbol(name, _) => (*name).into(),
        Expression::Quote(expression, _) => {
            // let expression = compile_expression(context, expression);
            sequence(allocate(
                context.allocator(),
                ["'".into(), compile_expression(context, expression)],
            ))
        }
    })
}

fn compile_expressions<'a, A: Allocator + Clone + 'a>(
    context: &mut Context<'a, A>,
    expressions: impl IntoIterator<Item = &'a Expression<'a, A>>,
) -> Document<'a> {
    let mut documents = Vec::new_in(context.allocator());
    let mut last_expression = None::<&Expression<A>>;

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

    sequence(allocate(context.allocator(), documents))
}

fn compile_line_comment<'a, A: Allocator + Clone + 'a>(
    context: &mut Context<'a, A>,
    position: &Position,
    document: impl Fn(&mut Context<'a, A>) -> Document<'a>,
) -> Document<'a> {
    let block_comment = compile_block_comment(context, position);
    let document = document(context);
    let suffix_comment = compile_suffix_comment(context, position);

    sequence(allocate(
        context.allocator(),
        [block_comment, document, suffix_comment],
    ))
}

fn compile_suffix_comment<'a, A: Allocator + Clone + 'a>(
    context: &mut Context<A>,
    position: &Position,
) -> Document<'a> {
    let allocator = context.allocator();

    sequence(allocate_vec(
        context.allocator(),
        context
            .drain_current_comment(get_line_index(context, position.start()))
            .map(|comment| {
                line_suffix(allocate(
                    allocator.clone(),
                    " ".to_owned() + COMMENT_PREFIX + comment.value().trim_end(),
                ))
            }),
    ))
}

fn compile_block_comment<'a, A: Allocator + Clone + 'a>(
    context: &mut Context<'a, A>,
    position: &Position,
) -> Document<'a> {
    let comments = allocate_vec(
        context.allocator(),
        context.drain_comments_before(get_line_index(context, position.start())),
    );

    compile_all_comments(
        context,
        comments,
        Some(get_line_index(context, position.start())),
    )
}

fn compile_remaining_block_comment<'a, A: Allocator + Clone + 'a>(
    context: &mut Context<'a, A>,
) -> Document<'a> {
    let comments = allocate_vec(
        context.allocator(),
        context.drain_comments_before(usize::MAX),
    );

    compile_all_comments(context, comments, None)
}

fn compile_all_comments<'a, A: Allocator + Clone + 'a>(
    context: &Context<A>,
    comments: &'a [&'a Comment<'a>],
    last_line_index: Option<usize>,
) -> Document<'a> {
    sequence(allocate_vec(
        context.allocator(),
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
                sequence(allocate(
                    context.allocator(),
                    [
                        COMMENT_PREFIX.into(),
                        comment.value().trim_end().into(),
                        r#break(allocate(context.allocator(), line())),
                        if get_line_index(context, comment.position().end() - 1) + 1
                            < next_line_index
                        {
                            line()
                        } else {
                            empty()
                        },
                    ],
                ))
            }),
    ))
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

fn allocate<'a, A: Allocator + 'a, T>(allocator: A, value: T) -> &'a T
where
    A: 'a,
{
    Box::leak(Box::new_in(value, allocator))
}

fn allocate_vec<'a, A: Allocator + 'a, T>(
    allocator: A,
    values: impl IntoIterator<Item = T>,
) -> &'a [T]
where
    A: 'a,
{
    let mut vec = Vec::new_in(allocator);

    vec.extend(values);

    Vec::leak(vec)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{position::Position, position_map::PositionMap};
    use indoc::indoc;
    use std::alloc::Global;

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
                &[],
                &PositionMap::new("(foo bar)"),
                Global,
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
                &[],
                &PositionMap::new("(foo\nbar)"),
                Global,
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
                &[],
                &PositionMap::new("'foo"),
                Global,
            ),
            "'foo\n"
        );
    }

    #[test]
    fn format_string() {
        assert_eq!(
            format::<Global>(
                &[Expression::String("foo", Position::new(0, 3))],
                &[],
                &[],
                &PositionMap::new("\"foo\""),
                Global,
            ),
            "\"foo\"\n"
        );
    }

    #[test]
    fn format_symbol() {
        assert_eq!(
            format::<Global>(
                &[Expression::Symbol("foo", Position::new(0, 3))],
                &[],
                &[],
                &PositionMap::new("foo"),
                Global,
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
                            Expression::Symbol("foo", Position::new(0, 1)),
                            Expression::Symbol("bar", Position::new(1, 2))
                        ],
                        Position::new(0, 1)
                    )],
                    &[],
                    &[],
                    &PositionMap::new("\na"),
                    Global,
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
                            Expression::Symbol("foo", Position::new(0, 1)),
                            Expression::Symbol("bar", Position::new(2, 3))
                        ],
                        Position::new(0, 1)
                    )],
                    &[],
                    &[],
                    &PositionMap::new("\n\na"),
                    Global,
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
                    &[],
                    &PositionMap::new("\n\n"),
                    Global,
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
                format::<Global>(
                    &[
                        Expression::Symbol("foo", Position::new(0, 1)),
                        Expression::Symbol("bar", Position::new(1, 2))
                    ],
                    &[],
                    &[],
                    &PositionMap::new("\na"),
                    Global,
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
                format::<Global>(
                    &[
                        Expression::Symbol("foo", Position::new(0, 1)),
                        Expression::Symbol("bar", Position::new(2, 3))
                    ],
                    &[],
                    &[],
                    &PositionMap::new("\n\na"),
                    Global,
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
                format::<Global>(
                    &[Expression::Symbol("foo", Position::new(1, 2))],
                    &[Comment::new("bar", Position::new(0, 1))],
                    &[],
                    &PositionMap::new("\na"),
                    Global,
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
                format::<Global>(
                    &[Expression::Symbol("foo", Position::new(2, 3))],
                    &[Comment::new("bar", Position::new(0, 1))],
                    &[],
                    &PositionMap::new("\n\na"),
                    Global,
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
                format::<Global>(
                    &[Expression::Symbol("foo", Position::new(3, 4))],
                    &[
                        Comment::new("bar", Position::new(0, 1)),
                        Comment::new("baz", Position::new(1, 2))
                    ],
                    &[],
                    &PositionMap::new("\n\n\na"),
                    Global,
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
                format::<Global>(
                    &[Expression::Symbol("foo", Position::new(4, 5))],
                    &[
                        Comment::new("bar", Position::new(0, 1)),
                        Comment::new("baz", Position::new(2, 3))
                    ],
                    &[],
                    &PositionMap::new("\n\n\n\na"),
                    Global,
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
        fn format_block_comment_after_expression() {
            assert_eq!(
                format::<Global>(
                    &[
                        Expression::Symbol("foo", Position::new(0, 1)),
                        Expression::Symbol("baz", Position::new(2, 3))
                    ],
                    &[Comment::new("bar", Position::new(1, 2))],
                    &[],
                    &PositionMap::new("\n\n\n"),
                    Global,
                ),
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
                ),
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
                format::<Global>(
                    &[Expression::Symbol("foo", Position::new(0, 1))],
                    &[Comment::new("bar", Position::new(0, 1))],
                    &[],
                    &PositionMap::new("\na"),
                    Global,
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
                format::<Global>(
                    &[Expression::Symbol("foo", Position::new(0, 1))],
                    &[
                        Comment::new("bar", Position::new(0, 1)),
                        Comment::new("baz", Position::new(0, 1))
                    ],
                    &[],
                    &PositionMap::new("\na"),
                    Global,
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
                        vec![Expression::Symbol("foo", Position::new(1, 2))],
                        Position::new(0, 1)
                    )],
                    &[Comment::new("bar", Position::new(0, 1))],
                    &[],
                    &PositionMap::new("\n\n"),
                    Global,
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
                format::<Global>(
                    &[Expression::Symbol("foo", Position::new(0, 1))],
                    &[Comment::new("bar", Position::new(1, 2))],
                    &[],
                    &PositionMap::new("\n\n"),
                    Global,
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
                format::<Global>(
                    &[Expression::Symbol("foo", Position::new(0, 1))],
                    &[
                        Comment::new("bar", Position::new(1, 2)),
                        Comment::new("baz", Position::new(2, 3))
                    ],
                    &[],
                    &PositionMap::new("\n\n\n"),
                    Global,
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
                format::<Global>(
                    &[Expression::Symbol("foo", Position::new(0, 1))],
                    &[
                        Comment::new("bar", Position::new(1, 2)),
                        Comment::new("baz", Position::new(3, 4))
                    ],
                    &[],
                    &PositionMap::new("\n\n\n\n"),
                    Global,
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

    mod hash_directive {
        use super::*;
        use pretty_assertions::assert_eq;

        #[test]
        fn format_hash_directive() {
            assert_eq!(
                format::<Global>(
                    &[],
                    &[],
                    &[HashDirective::new("foo", Position::new(0, 0))],
                    &PositionMap::new("\n"),
                    Global,
                ),
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
                format::<Global>(
                    &[],
                    &[],
                    &[
                        HashDirective::new("foo", Position::new(0, 0)),
                        HashDirective::new("bar", Position::new(2, 2))
                    ],
                    &PositionMap::new("\n\n\n"),
                    Global,
                ),
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
                format::<Global>(
                    &[Expression::Symbol("bar", Position::new(0, 0))],
                    &[],
                    &[HashDirective::new("foo", Position::new(0, 0))],
                    &PositionMap::new("\n"),
                    Global,
                ),
                indoc!(
                    "
                    #foo

                    bar
                    "
                )
            );
        }
    }
}
