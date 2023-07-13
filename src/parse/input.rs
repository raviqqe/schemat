use nom_locate::LocatedSpan;
use std::alloc::Allocator;

pub type Input<'a, A: Allocator> = LocatedSpan<&'a str, A>;
