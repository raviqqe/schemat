#![allow(unstable_name_collisions)]

use crate::{ast::Expression, context::Context, position_map::PositionMap};
use itertools::Itertools;
use mfmt::{flatten, indent, line, r#break, sequence, Document};

pub fn format(module: &[Expression], position_map: &PositionMap) -> String {
    let context = Context::new(position_map);
    mfmt::format(&compile_module(&context, module))
}

fn compile_module(context: &Context, module: &[Expression]) -> Document {
    sequence(
        module
            .iter()
            .zip(
                module
                    .iter()
                    .map(|expression| expression.position().start()),
            )
            .map(|expression| sequence([compile_expression(context, expression), line()])),
    )
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
                    .chain([compile_expressions(context, &first)])
                    .chain(if last.is_empty() {
                        None
                    } else {
                        Some(r#break(indent(sequence([
                            line(),
                            compile_expressions(context, &last),
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

fn compile_expressions(context: &Context, expressions: &[&Expression]) -> Document {
    sequence(
        expressions
            .iter()
            .map(|expression| compile_expression(context, expression))
            .intersperse(line()),
    )
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
                &PositionMap::new(""),
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
                &PositionMap::new(""),
            ),
            "'foo\n"
        );
    }

    #[test]
    fn format_string() {
        assert_eq!(
            format(
                &[Expression::String("foo", Position::new(0, 3))],
                &PositionMap::new(""),
            ),
            "\"foo\"\n"
        );
    }

    #[test]
    fn format_symbol() {
        assert_eq!(
            format(
                &[Expression::Symbol("foo", Position::new(0, 3))],
                &PositionMap::new(""),
            ),
            "foo\n"
        );
    }

    #[test]
    fn keep_no_blank_line() {
        assert_eq!(
            format(
                &[
                    Expression::Symbol("foo", Position::new(0, 2)),
                    Expression::Symbol("bar", Position::new(1, 2))
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
