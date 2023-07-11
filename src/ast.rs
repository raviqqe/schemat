#[derive(Debug, Eq, PartialEq)]
pub enum Expression<'a> {
    List(Vec<Expression<'a>>),
    Quote(Box<Expression<'a>>),
    Symbol(&'a str),
}
