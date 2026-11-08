#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Side {
    White,
    Black,
}

impl Side {
    pub const fn opponent(self) -> Self {
        match self {
            Self::White => Self::Black,
            Self::Black => Self::White,
        }
    }

    /// The rank direction this side's pawns walk: up the board for White.
    pub(crate) const fn forward(self) -> i8 {
        match self {
            Self::White => 1,
            Self::Black => -1,
        }
    }

    pub(crate) const fn back_rank(self) -> u8 {
        match self {
            Self::White => 0,
            Self::Black => 7,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PieceKind {
    King,
    Queen,
    Rook,
    Bishop,
    Knight,
    Pawn,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Piece {
    pub side: Side,
    pub kind: PieceKind,
}

impl Piece {
    pub const fn new(side: Side, kind: PieceKind) -> Self {
        Self { side, kind }
    }
}
