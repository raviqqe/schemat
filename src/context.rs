use crate::{ast::Comment, position::Position, position_map::PositionMap};

pub struct Context<'a> {
    comments: Vec<&'a Comment<'a>>,
    position_map: &'a PositionMap,
}

impl<'a> Context<'a> {
    pub fn new(comments: &'a [Comment<'a>], position_map: &'a PositionMap) -> Self {
        Self {
            comments: comments.iter().collect(),
            position_map,
        }
    }

    pub fn position_map(&self) -> &'a PositionMap {
        self.position_map
    }

    pub fn drain_comments_before<'b>(
        &'b mut self,
        line_index: usize,
    ) -> impl Iterator<Item = &'a Comment<'a>> + 'b {
        // This is O(n) and slow. Ha ha!
        self.comments.splice(
            ..self
                .comments
                .iter()
                .position(|comment| self.line_index(comment.position()) >= line_index)
                .unwrap_or(self.comments.len()),
            [],
        )
    }

    pub fn drain_current_comment(
        &mut self,
        line_index: usize,
    ) -> impl Iterator<Item = &'a Comment<'a>> + '_ {
        self.drain_comments_before(line_index + 1)
    }

    pub fn peek_comments_before<'b>(
        &'b self,
        line_index: usize,
    ) -> impl Iterator<Item = &'a Comment<'a>> + 'b {
        self.comments[..self
            .comments
            .iter()
            .position(|comment| self.line_index(comment.position()) >= line_index)
            .unwrap_or(self.comments.len())]
            .iter()
            .copied()
    }

    fn line_index(&self, position: &Position) -> usize {
        self.position_map()
            .line_index(position.start())
            .expect("valid offset")
    }
}
