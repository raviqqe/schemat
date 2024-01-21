use crate::{ast::Comment, position::Position, position_map::PositionMap};
use mfmt::Builder;
use std::{alloc::Allocator, collections::VecDeque};

pub struct Context<'a, A: Allocator + Clone> {
    comments: VecDeque<&'a Comment<'a>>,
    position_map: &'a PositionMap,
    builder: Builder<A>,
}

impl<'a, A: Allocator + Clone> Context<'a, A> {
    pub fn new(
        comments: &'a [Comment<'a>],
        position_map: &'a PositionMap,
        builder: Builder<A>,
    ) -> Self {
        Self {
            comments: comments.iter().collect(),
            position_map,
            builder,
        }
    }

    pub fn position_map(&self) -> &'a PositionMap {
        self.position_map
    }

    pub fn builder(&self) -> &Builder<A> {
        &self.builder
    }

    pub fn drain_comments<'b>(
        &'b mut self,
        line_index: usize,
    ) -> impl Iterator<Item = &'a Comment<'a>> + 'b {
        self.comments.drain(
            ..self
                .comments
                .iter()
                .position(|comment| self.line_index(comment.position()) >= line_index)
                .unwrap_or(self.comments.len()),
        )
    }

    pub fn drain_current_line_comment(
        &mut self,
        line_index: usize,
    ) -> impl Iterator<Item = &'a Comment<'a>> + '_ {
        self.drain_comments(line_index + 1)
    }

    pub fn peek_comments(&self, line_index: usize) -> impl Iterator<Item = &Comment> {
        self.comments
            .range(
                ..self
                    .comments
                    .iter()
                    .position(|comment| self.line_index(comment.position()) >= line_index)
                    .unwrap_or(self.comments.len()),
            )
            .copied()
    }

    fn line_index(&self, position: &Position) -> usize {
        self.position_map()
            .line_index(position.start())
            .expect("valid offset")
    }
}
