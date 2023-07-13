use crate::position::Position;

#[derive(Debug, Eq, PartialEq)]
pub struct HashDirective<'a> {
    value: &'a str,
    position: Position,
}

impl<'a> HashDirective<'a> {
    pub fn new(value: &'a str, position: Position) -> Self {
        Self { value, position }
    }

    pub fn value(&self) -> &'a str {
        self.value
    }
}
