use super::input::Input;
use crate::position_map::PositionMap;
use core::str;
use nom::error::{VerboseError, VerboseErrorKind};
use std::alloc::Allocator;

pub type NomError<'a, A> = VerboseError<Input<'a, A>>;

#[derive(Debug, PartialEq, Eq)]
pub struct ParseError {
    message: String,
    offset: usize,
}

impl ParseError {
    pub fn new<A: Allocator>(source: &str, error: nom::Err<NomError<'_, A>>) -> Self {
        match error {
            nom::Err::Incomplete(_) => Self::unexpected_end(source),
            nom::Err::Error(error) | nom::Err::Failure(error) => {
                let context = error
                    .errors
                    .iter()
                    .find_map(|(_, kind)| {
                        if let VerboseErrorKind::Context(context) = kind {
                            Some(context)
                        } else {
                            None
                        }
                    })
                    .copied();

                if let Some(&(ref input, _)) = error.errors.first() {
                    Self {
                        message: if let Some(character) =
                            error.errors.iter().find_map(|(_, kind)| {
                                if let VerboseErrorKind::Char(character) = kind {
                                    Some(character)
                                } else {
                                    None
                                }
                            }) {
                            [format!("'{character}' expected")]
                                .into_iter()
                                .chain(context.map(|context| format!("in {context}")))
                                .collect::<Vec<_>>()
                                .join(" ")
                        } else {
                            ["failed to parse"]
                                .into_iter()
                                .chain(context)
                                .collect::<Vec<_>>()
                                .join(" ")
                        },
                        offset: input.location_offset(),
                    }
                } else {
                    Self::unexpected_end(source)
                }
            }
        }
    }

    fn unexpected_end(source: &str) -> Self {
        Self {
            message: "unexpected end of source".into(),
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

    #[test]
    fn to_string() {
        let source = "foo";
        let position_map = PositionMap::new(source);

        let error = ParseError::new(
            "foo",
            nom::Err::Error(VerboseError {
                errors: vec![(Input::new("foo"), VerboseErrorKind::Context("bar"))],
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
