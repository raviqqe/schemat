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

    pub fn line(&self, offset: usize) -> usize {
        foo
    }
}
