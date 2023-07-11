#[derive(Debug, Eq, PartialEq)]
pub enum Expression<'a> {
    Symbol(&'a str),
    List(Vec<Expression<'a>>),
}
