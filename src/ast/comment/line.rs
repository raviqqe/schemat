use crate::position::Position;

#[derive(Debug, Eq, PartialEq)]
pub struct LineComment<'a> {
    content: &'a str,
    position: Position,
}

impl<'a> LineComment<'a> {
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
