#![allow(unstable_name_collisions)]

use crate::{ast::Expression, context::Context, position_map::PositionMap};
use mfmt::{flatten, indent, line, r#break, sequence, Document};

pub fn format(module: &[Expression], position_map: &PositionMap) -> String {
    let context = Context::new(position_map);
    mfmt::format(&compile_module(&context, module))
}

fn compile_module(context: &Context, module: &[Expression]) -> Document {
    sequence([compile_expressions(context, module), line()])
}

fn compile_expression(context: &Context, expression: &Expression) -> Document {
    match expression {
        Expression::List(expressions, position) => {
            let line_index = get_line_index(context, position.start());
            let (first, last) = expressions.iter().partition::<Vec<_>, _>(|expression| {
                get_line_index(context, expression.position().start()) == line_index
            });
            let extra_line = if let (Some(first), Some(last)) = (first.last(), last.first()) {
                if has_extra_line(context, first, last) {
                    Some(line())
                } else {
                    None
                }
            } else {
                None
            };

            flatten(sequence(
                ["(".into()]
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
    }
}

fn compile_expressions<'a>(
    context: &Context,
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

fn has_extra_line(
    context: &Context,
    last_expression: &Expression,
    expression: &Expression,
) -> bool {
    get_line_index(context, expression.position().start())
        .saturating_sub(get_line_index(context, last_expression.position().end()))
        > 1
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
}
