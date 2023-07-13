use nom_locate::LocatedSpan;

pub type Input<'a, A> = LocatedSpan<&'a str, A>;
