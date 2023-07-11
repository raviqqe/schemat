#[derive(Debug, Eq, PartialEq)]
pub enum Expression<'a> {
    List(Vec<Expression<'a>>, Position),
    Quote(Box<Expression<'a>>, Position),
    String(&'a str, Position),
    Symbol(&'a str, Position),
}
