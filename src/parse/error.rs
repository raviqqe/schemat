use super::input::Input;
use nom::error::VerboseError;

pub type Error<'a> = VerboseError<Input<'a>>;
