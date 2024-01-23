mod block;
mod line;

pub use self::{block::BlockComment, line::LineComment};
use crate::position::Position;

#[derive(Debug, Eq, PartialEq)]
pub enum Comment<'a> {
    Block(BlockComment<'a>),
    Line(LineComment<'a>),
}

impl<'a> Comment<'a> {
    pub fn content(&self) -> &str {
        match self {
            Self::Block(comment) => comment.content(),
            Self::Line(comment) => comment.content(),
        }
    }

    pub fn position(&self) -> &Position {
        match self {
            Self::Block(comment) => comment.position(),
            Self::Line(comment) => comment.position(),
        }
    }
}

impl<'a> From<BlockComment<'a>> for Comment<'a> {
    fn from(comment: BlockComment<'a>) -> Self {
        Self::Block(comment)
    }
}

impl<'a> From<LineComment<'a>> for Comment<'a> {
    fn from(comment: LineComment<'a>) -> Self {
        Self::Line(comment)
    }
}
