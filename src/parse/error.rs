use super::input::Input;
use crate::position_map::PositionMap;
use core::str;
use nom::{error::Error, Needed};
use std::alloc::Allocator;

pub type NomError<'a, A> = Error<Input<'a, A>>;

#[derive(Debug, PartialEq, Eq)]
pub struct ParseError {
    message: &'static str,
    offset: usize,
}

impl ParseError {
    pub fn new<A: Allocator>(source: &str, error: nom::Err<NomError<'_, A>>) -> Self {
        Self {
            message: match error {
                nom::Err::Incomplete(_) => "parsing requires more data",
                nom::Err::Error(error) | nom::Err::Failure(error) => "failed to parse",
            },
            offset: source.as_bytes().len() - 1,
        }
    }

    pub fn to_string(&self, source: &str, position_map: &PositionMap) -> String {
        let bytes = &source.as_bytes()[position_map.line_range(self.offset).expect("valid offset")];

        format!(
            "{}\n{}:{}: {}",
            &self.message,
            &position_map.line_index(self.offset).expect("valid offset") + 1,
            &position_map
                .column_index(self.offset)
                .expect("valid offset")
                + 1,
            String::from_utf8_lossy(bytes).trim_end(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use indoc::indoc;
    use pretty_assertions::assert_eq;
    use std::alloc::Global;

    #[test]
    fn to_string() {
        let source = "foo";
        let position_map = PositionMap::new(source);

        let error = ParseError::new(
            "foo",
            nom::Err::Error(Error {
                input: Input::new_extra("foo", Global),
                code: foo,
            }),
        );

        assert_eq!(
            error.to_string(source, &position_map),
            indoc!(
                "
                    failed to parse bar
                    1:1: foo
                "
            )
            .trim()
        );
    }
}
