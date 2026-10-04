#![doc = include_str!("../README.md")]

extern crate alloc;

mod ast;
mod context;
mod error;
mod file;
mod format;
mod parse;
mod position;
mod position_map;

pub use self::{
    error::ApplicationError,
    file::{display_path, read_paths},
};
use self::{
    format::format,
    parse::{ParseError, parse, parse_comments, parse_hash_directives},
    position_map::PositionMap,
};
use bumpalo::Bump;

/// Formats a source string.
pub fn format_string(source: &str) -> Result<String, ApplicationError> {
    let position_map = PositionMap::new(source);
    let convert_error = |error| convert_parse_error(error, source, &position_map);
    let allocator = Bump::new();

    Ok(format(
        &parse(source, &allocator).map_err(convert_error)?,
        &parse_comments(source, &allocator).map_err(convert_error)?,
        &parse_hash_directives(source, &allocator).map_err(convert_error)?,
        &position_map,
        &allocator,
    )?)
}

fn convert_parse_error(
    error: ParseError,
    source: &str,
    position_map: &PositionMap,
) -> ApplicationError {
    ApplicationError::Parse(error.to_string(source, position_map))
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn format_empty_string() {
        assert_eq!(format_string("").unwrap(), "\n");
    }

    #[test]
    fn format_list() {
        assert_eq!(format_string("( foo  bar )").unwrap(), "(foo bar)\n");
    }

    #[test]
    fn fail_to_format_unclosed_list() {
        assert!(matches!(
            format_string("(foo"),
            Err(ApplicationError::Parse(_))
        ));
    }
}
