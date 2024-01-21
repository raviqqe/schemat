mod block;
mod line;

pub use self::{block::BlockComment, line::LineComment};

#[derive(Debug, Eq, PartialEq)]
pub enum Comment<'a> {
    Block(BlockComment<'a>),
    Line(LineComment<'a>),
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
