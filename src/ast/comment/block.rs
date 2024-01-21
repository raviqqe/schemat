use crate::position::Position;

#[derive(Debug, Eq, PartialEq)]
pub struct BlockComment<'a> {
    value: &'a str,
    position: Position,
}

impl<'a> BlockComment<'a> {
    pub fn new(value: &'a str, position: Position) -> Self {
        Self { value, position }
    }

    pub fn value(&self) -> &'a str {
        self.value
    }

    pub fn position(&self) -> &Position {
        &self.position
    }
}
