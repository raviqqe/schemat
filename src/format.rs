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
            let line_index = context.position_map().line_index(position.start());

            let (first, last) = expressions.iter().partition::<Vec<_>, _>(|expression| {
                context
                    .position_map()
                    .line_index(expression.position().start())
                    == line_index
            });

            flatten(sequence(
                ["(".into()]
                    .into_iter()
                    .chain([compile_expressions(context, first)])
                    .chain(if last.is_empty() {
                        None
                    } else {
                        Some(r#break(indent(sequence([
                            line(),
                            compile_expressions(context, last),
                        ]))))
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
            let current_line = get_line_index(context, expression);
            let last_line = get_line_index(context, last_expression);
            let difference = current_line.saturating_sub(last_line);

            documents.push(line());
            documents.extend(if difference <= 1 { None } else { Some(line()) });
        }

        documents.push(compile_expression(context, expression));

        last_expression = Some(expression);
    }

    sequence(documents)
}

fn get_line_index(context: &Context, expression: &Expression) -> usize {
    context
        .position_map()
        .line_index(expression.position().start())
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
                        Expression::Symbol("bar", Position::new(6, 9))
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
                    Expression::Symbol("foo", Position::new(0, 2)),
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
