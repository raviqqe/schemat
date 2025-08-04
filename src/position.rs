#[derive(Debug, Eq, PartialEq)]
pub struct Position {
    start: usize,
    end: usize,
}

impl Position {
    pub const fn new(start: usize, end: usize) -> Self {
        Self { start, end }
    }

    pub const fn start(&self) -> usize {
        self.start
    }

    pub const fn end(&self) -> usize {
        self.end
    }

    pub const fn set_start(&self, start: usize) -> Self {
        Self {
            start,
            end: self.end,
        }
    }

    pub const fn set_end(&self, end: usize) -> Self {
        Self {
            start: self.start,
            end,
        }
    }
}
