use crate::position::Position;

#[derive(Debug, Eq, PartialEq)]
pub struct BlockComment<'a> {
    content: &'a str,
    position: Position,
}

impl<'a> BlockComment<'a> {
    pub fn new(content: &'a str, position: Position) -> Self {
        Self { content, position }
    }

    pub fn content(&self) -> &'a str {
        self.content
    }

    pub fn position(&self) -> &Position {
        &self.position
    }
}
