#![allow(unstable_name_collisions)]

use crate::ast::Expression;
use itertools::Itertools;
use mfmt::{line, sequence, Document};

pub fn format(module: &[Expression]) -> String {
    mfmt::format(&compile_module(module))
}

fn compile_module(module: &[Expression]) -> Document {
    sequence(module.iter().map(compile_expression).intersperse(line()))
}

fn compile_expression(expression: &Expression) -> Document {
    match expression {
        Expression::String(string) => sequence(["\"", string, "\""]),
        Expression::Symbol(name) => (*name).into(),
        _ => todo!(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_string() {
        assert_eq!(format(&[Expression::String("foo")]), "\"foo\"");
    }

    #[test]
    fn format_symbol() {
        assert_eq!(format(&[Expression::Symbol("foo")]), "foo");
    }
}
