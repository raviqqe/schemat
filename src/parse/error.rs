use super::input::Input;
use nom::error::{VerboseError, VerboseErrorKind};

pub type NomError<'a> = VerboseError<Input<'a>>;

#[derive(Debug, PartialEq, Eq)]
pub struct ParseError {
    message: String,
    offset: usize,
}

impl ParseError {
    pub fn new(source: &str, error: nom::Err<NomError<'_>>) -> Self {
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

                if let Some(&(input, _)) = error.errors.first() {
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

    pub fn offset(&self) -> usize {
        self.offset
    }

    fn unexpected_end(source: &str) -> Self {
        Self {
            message: "unexpected end of source".into(),
            offset: source.as_bytes().len() - 1,
        }
    }
}
