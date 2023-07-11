#![allow(unstable_name_collisions)]

use crate::ast::Expression;
use itertools::Itertools;
use mfmt::{flatten, line, sequence, Document};

pub fn format(module: &[Expression]) -> String {
    mfmt::format(&compile_module(module))
}

fn compile_module(module: &[Expression]) -> Document {
    sequence(module.iter().map(compile_expression).intersperse(line()))
}

fn compile_expression(expression: &Expression) -> Document {
    match expression {
        Expression::List(expressions, _) => flatten(sequence(
            ["(".into()]
                .into_iter()
                .chain(
                    expressions
                        .iter()
                        .map(compile_expression)
                        .intersperse(line()),
                )
                .chain([")".into()])
                .collect::<Vec<_>>(),
        )),
        Expression::String(string, _) => sequence(["\"", string, "\""]),
        Expression::Symbol(name, _) => (*name).into(),
        Expression::Quote(expression, _) => sequence(["'".into(), compile_expression(expression)]),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::position::Position;

    #[test]
    fn format_list() {
        assert_eq!(
            format(&[Expression::List(
                vec![
                    Expression::Symbol("foo", Position::new(0, 2)),
                    Expression::Symbol("bar", Position::new(0, 2))
                ],
                Position::new(0, 2)
            )]),
            "(foo bar)"
        );
    }

    #[test]
    fn format_quote() {
        assert_eq!(
            format(&[Expression::Quote(
                Expression::Symbol("foo", Position::new(0, 3)).into(),
                Position::new(0, 3)
            )]),
            "'foo"
        );
    }

    #[test]
    fn format_string() {
        assert_eq!(
            format(&[Expression::String("foo", Position::new(0, 3))]),
            "\"foo\""
        );
    }

    #[test]
    fn format_symbol() {
        assert_eq!(
            format(&[Expression::Symbol("foo", Position::new(0, 3))]),
            "foo"
        );
    }
}
