//! A FEN that could not be read.

#[must_use]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FenError {
    MissingField,
    Placement,
    Piece(char),
    SideToMove,
    Castling(char),
    EnPassant,
    MoveCounter,
}

impl std::fmt::Display for FenError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingField => {
                f.write_str("a FEN needs placement, side, castling and en passant")
            }
            Self::Placement => f.write_str("the placement must describe 8 ranks of 8 squares"),
            Self::Piece(c) => write!(f, "{c:?} is not a piece letter"),
            Self::SideToMove => f.write_str("the side to move must be w or b"),
            Self::Castling(c) => write!(f, "{c:?} is not a castling right"),
            Self::EnPassant => f.write_str("en passant must be - or a square"),
            Self::MoveCounter => f.write_str("the move counters must be whole numbers"),
        }
    }
}

impl std::error::Error for FenError {}
