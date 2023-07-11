#[derive(Debug)]
pub enum Expression<'a> {
    Symbol(&'a str),
    List(Vec<Expression<'a>>),
}
