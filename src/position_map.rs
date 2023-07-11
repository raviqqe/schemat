pub struct PositionMap {
    lines: Vec<usize>,
}

impl PositionMap {
    pub fn new(source: &str) -> Self {
        let lines = vec![];

        for character in source.iter().enumerate() {
            if character == '\n' {
                lines.push(offset);
            }
        }

        Self { lines }
    }

    pub fn line(&self, offset: usize) -> Option<usize> {
        lines.binary_search(offset).ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn get_line_of_empty_source() {
        let source = "";
        let map = PositionMap::new(source);

        map.line(0, 1, 2)
    }
}
