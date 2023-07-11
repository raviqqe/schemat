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
        Expression::Symbol(name) => (*name).into(),
        _ => todo!(),
    }
}
