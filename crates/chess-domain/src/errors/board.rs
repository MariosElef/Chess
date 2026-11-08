//! A square name that could not be read.

#[must_use]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ParseSquareError;

impl std::fmt::Display for ParseSquareError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("not a square name like e4")
    }
}

impl std::error::Error for ParseSquareError {}
