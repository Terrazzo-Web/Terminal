/// Represents the size of the pty.
#[derive(Debug, Clone, Copy)]
pub struct Size {
    row: u16,
    col: u16,
    xpixel: u16,
    ypixel: u16,
}

impl Size {
    /// Returns a [`Size`] instance with the given number of rows and
    /// columns.
    #[must_use]
    pub fn new(row: u16, col: u16) -> Self {
        Self::new_with_pixel(row, col, 0, 0)
    }

    /// Returns a [`Size`] instance with the given number of rows and
    /// columns, as well as the given pixel dimensions.
    #[must_use]
    pub fn new_with_pixel(row: u16, col: u16, xpixel: u16, ypixel: u16) -> Self {
        Self {
            row,
            col,
            xpixel,
            ypixel,
        }
    }
}

impl From<Size> for libc::winsize {
    fn from(size: Size) -> Self {
        Self {
            ws_row: size.row,
            ws_col: size.col,
            ws_xpixel: size.xpixel,
            ws_ypixel: size.ypixel,
        }
    }
}
