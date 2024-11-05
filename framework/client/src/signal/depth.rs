#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Depth {
    depth: i32,
}

impl Depth {
    pub fn zero() -> Self {
        Self { depth: 0 }
    }

    pub fn next(self) -> Self {
        Self {
            depth: self.depth + 1,
        }
    }
}
