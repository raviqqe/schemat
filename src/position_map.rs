pub struct PositionMap {
    lines: Vec<usize>,
}

impl PositionMap {
    pub fn new(source: &str) -> Self {
        let mut lines = vec![];

        for (index, character) in source.chars().enumerate() {
            if character == '\n' {
                lines.push(index);
            }
        }

        Self { lines }
    }

    pub fn line(&self, offset: usize) -> Option<usize> {
        self.lines.binary_search(&offset).ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn get_line_of_empty_source() {
        let source = "";
        let map = PositionMap::new(source);

        assert_eq!(map.line(0), Some(0));
        assert_eq!(map.line(1), Some(0));
        assert_eq!(map.line(2), Some(0));
        assert_eq!(map.line(0), Some(0));
    }
}
