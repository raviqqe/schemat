mod parser;

use self::parser::{module, Span};
use crate::ast::Expression;

pub fn parse(source: &str) -> Result<Vec<Expression>, ()> {
    let input = Span::new(source);

    module(input).map(|(_, module)| module)
}
