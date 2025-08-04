use crate::position::Position;

#[derive(Debug, Eq, PartialEq)]
pub struct HashDirective<'a> {
    value: &'a str,
    position: Position,
}

impl<'a> HashDirective<'a> {
    pub const fn new(value: &'a str, position: Position) -> Self {
        Self { value, position }
    }

    pub const fn value(&self) -> &'a str {
        self.value
    }
}
