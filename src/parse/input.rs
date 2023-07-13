use nom_locate::LocatedSpan;

pub type Input<'a, A: Allocator> = LocatedSpan<&'a str, A>;
