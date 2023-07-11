#[derive(Debug)]
pub struct PositionMap {
    lines: Vec<usize>,
}

impl PositionMap {
    pub fn new(source: &str) -> Self {
        let mut lines = vec![0];

        for (index, &character) in source.as_bytes().iter().enumerate() {
            if character == b'\n' {
                lines.push(index + 1);
            }
        }

        // Add an implicit newline character.
        if source.len() > *lines.iter().last().unwrap() {
            lines.push(source.len() );
        }

        Self { lines }
    }

    pub fn line(&self, offset: usize) -> Option<usize> {
        let line = match self.lines.binary_search(&offset) {
            Ok(line) => line,
            Err(line) => line - 1,
        };

        if line >= self.lines.len() - 1 {
            None
        } else {
            Some(line)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slice() {
        assert_eq!([0].binary_search(&0), Ok(0));
        assert_eq!([0, 1].binary_search(&0), Ok(0));
        assert_eq!([0, 1].binary_search(&1), Ok(1));
        assert_eq!([3].binary_search(&0), Err(0));
        assert_eq!([3].binary_search(&1), Err(0));
        assert_eq!([3].binary_search(&2), Err(0));
        assert_eq!([3].binary_search(&3), Ok(0));
    }

    #[test]
    fn get_line_of_empty_source() {
        let source = "";
        let map = PositionMap::new(source);

        assert_eq!(map.line(0), None);
        assert_eq!(map.line(1), None);
    }

    #[test]
    fn get_line_in_line() {
        let source = "foo";
        let map = PositionMap::new(source);

        assert_eq!(map.line(0), Some(0));
        assert_eq!(map.line(1), Some(0));
        assert_eq!(map.line(2), Some(0));
        assert_eq!(map.line(3), None);
    }

    #[test]
    fn get_line_in_two_lines() {
        let source = "foo\nbar\n";
        let map = PositionMap::new(source);

        assert_eq!(map.line(0), Some(0));
        assert_eq!(map.line(1), Some(0));
        assert_eq!(map.line(2), Some(0));
        assert_eq!(map.line(3), Some(0));
        assert_eq!(map.line(4), Some(1));
        assert_eq!(map.line(5), Some(1));
        assert_eq!(map.line(6), Some(1));
        assert_eq!(map.line(7), Some(1));
        assert_eq!(map.line(8), None);
    }
}
