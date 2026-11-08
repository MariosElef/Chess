use crate::{PieceKind, Square};

/// A piece going from one square to another; `promotion` is set only for a pawn reaching the last rank.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Move {
    pub from: Square,
    pub to: Square,
    pub promotion: Option<PieceKind>,
}

impl Move {
    pub const fn new(from: Square, to: Square) -> Self {
        Self {
            from,
            to,
            promotion: None,
        }
    }

    pub const fn promoting(from: Square, to: Square, kind: PieceKind) -> Self {
        Self {
            from,
            to,
            promotion: Some(kind),
        }
    }
}

/// UCI notation, as engines read and write it: `e2e4`, `e7e8q`.
impl std::fmt::Display for Move {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}{}", self.from, self.to)?;
        match self.promotion {
            Some(PieceKind::Queen) => f.write_str("q"),
            Some(PieceKind::Rook) => f.write_str("r"),
            Some(PieceKind::Bishop) => f.write_str("b"),
            Some(PieceKind::Knight) => f.write_str("n"),
            Some(PieceKind::King | PieceKind::Pawn) | None => Ok(()),
        }
    }
}
