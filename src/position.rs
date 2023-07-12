#[derive(Debug, Eq, PartialEq)]
pub struct Position {
    start: usize,
    end: usize,
}

impl Position {
    pub fn new(start: usize, end: usize) -> Self {
        Self { start, end }
    }

    pub fn start(&self) -> usize {
        self.start
    }

    pub fn end(&self) -> usize {
        self.end
    }
}
