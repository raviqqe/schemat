use mfmt::{flatten, indent, r#break, sequence, Document};
use std::alloc::Allocator;

#[derive(Clone, Debug)]
pub struct Builder<A: Allocator> {
    allocator: A,
}

impl<'a, A: Allocator + Clone + 'a> Builder<A> {
    pub fn new(allocator: A) -> Self {
        Self { allocator }
    }

    pub fn allocator(&self) -> &A {
        &self.allocator
    }

    pub fn sequence(
        &self,
        values: impl IntoIterator<Item = impl Into<Document<'a>>>,
    ) -> Document<'a> {
        sequence(self.allocate_slice(values.into_iter().map(Into::into)))
    }

    pub fn flatten(&self, value: impl Into<Document<'a>>) -> Document<'a> {
        flatten(self.allocate(value.into()))
    }

    pub fn indent(&self, value: impl Into<Document<'a>>) -> Document<'a> {
        indent(self.allocate(value.into()))
    }

    pub fn r#break(&self, value: impl Into<Document<'a>>) -> Document<'a> {
        r#break(self.allocate(value.into()))
    }

    pub fn allocate<T>(&self, value: T) -> &'a T {
        Box::leak(Box::new_in(value, self.allocator.clone()))
    }

    pub fn allocate_slice<T>(&self, values: impl IntoIterator<Item = T>) -> &'a [T] {
        let mut vec = Vec::new_in(self.allocator.clone());

        vec.extend(values);

        Vec::leak(vec)
    }
}
