use std::ops::Mul;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Size {
    pub x: i32,
    pub y: i32,
}

impl Mul<f64> for Size {
    type Output = Size;

    fn mul(self, f: f64) -> Size {
        Size {
            x: (self.x as f64 * f) as i32,
            y: (self.y as f64 * f) as i32,
        }
    }
}
