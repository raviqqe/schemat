use crate::position_map::PositionMap;
use core::str;

#[derive(Debug, PartialEq, Eq)]
pub struct ParseError {
    offset: usize,
}

impl ParseError {
    pub fn new(source: &str, offset: usize) -> Self {
        Self {
            offset: offset.min(source.floor_char_boundary(source.len() - 1)),
        }
    }

    pub fn to_string(&self, source: &str, position_map: &PositionMap) -> String {
        let bytes = &source.as_bytes()[position_map.line_range(self.offset).expect("valid offset")];

        format!(
            "failed to parse at line {} and column {}: {}",
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
    use pretty_assertions::assert_eq;

    #[test]
    fn to_string() {
        let source = "foo";
        let position_map = PositionMap::new(source);

        assert_eq!(
            ParseError::new(source, 0).to_string(source, &position_map),
            "failed to parse at line 1 and column 1: foo"
        );
    }
}
