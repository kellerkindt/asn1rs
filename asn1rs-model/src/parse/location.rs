#[derive(Debug, Default, Copy, Clone, PartialOrd, PartialEq, Eq)]
pub struct Location {
    line: usize,
    column: usize,
}

impl Location {
    pub const fn at(line: usize, column: usize) -> Location {
        Self { line, column }
    }

    pub const fn line(&self) -> usize {
        self.line
    }

    pub const fn column(&self) -> usize {
        self.column
    }
}
