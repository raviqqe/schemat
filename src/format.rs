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
        Expression::List(expressions) => flatten(sequence(
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
        Expression::String(string) => sequence(["\"", string, "\""]),
        Expression::Symbol(name) => (*name).into(),
        Expression::Quote(expression) => sequence(["'".into(), compile_expression(expression)]),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_list() {
        assert_eq!(
            format(&[Expression::List(vec![
                Expression::Symbol("foo"),
                Expression::Symbol("bar")
            ])]),
            "(foo bar)"
        );
    }

    #[test]
    fn format_quote() {
        assert_eq!(
            format(&[Expression::Quote(Expression::Symbol("foo").into())]),
            "'foo"
        );
    }

    #[test]
    fn format_string() {
        assert_eq!(format(&[Expression::String("foo")]), "\"foo\"");
    }

    #[test]
    fn format_symbol() {
        assert_eq!(format(&[Expression::Symbol("foo")]), "foo");
    }
}
