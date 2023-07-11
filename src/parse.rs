mod parser;

use self::parser::{module, Span};
use crate::ast::Expression;
use std::error::Error;

pub fn parse(source: &str) -> Result<Vec<Expression>, Box<dyn Error + '_>> {
    Ok(module(Span::new(source)).map(|(_, module)| module)?)
}
