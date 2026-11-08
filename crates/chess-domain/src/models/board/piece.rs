use crate::{PieceKind, Side};

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
