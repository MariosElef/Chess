use crate::{ParseMoveError, PieceKind, Square};

/// A piece going from one square to another; `promotion` only for a pawn reaching the last rank.
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

/// Reads UCI notation back: `e2e4`, or `e7e8q` for a promotion.
impl std::str::FromStr for Move {
    type Err = ParseMoveError;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        if !text.is_ascii() || !(4..=5).contains(&text.len()) {
            return Err(ParseMoveError::Length);
        }
        let from = text[0..2].parse().map_err(ParseMoveError::Square)?;
        let to = text[2..4].parse().map_err(ParseMoveError::Square)?;
        match text[4..].chars().next() {
            None => Ok(Self::new(from, to)),
            Some(letter) => Ok(Self::promoting(from, to, promotion_kind(letter)?)),
        }
    }
}

fn promotion_kind(letter: char) -> Result<PieceKind, ParseMoveError> {
    match letter {
        'q' => Ok(PieceKind::Queen),
        'r' => Ok(PieceKind::Rook),
        'b' => Ok(PieceKind::Bishop),
        'n' => Ok(PieceKind::Knight),
        _ => Err(ParseMoveError::Promotion(letter)),
    }
}

#[cfg(test)]
mod tests {
    use super::Move;
    use crate::{ParseMoveError, PieceKind};

    #[test]
    fn uci_text_reads_back_into_the_same_move() {
        for text in ["e2e4", "g1f3", "e7e8q", "a2a1n"] {
            let mv: Move = text.parse().expect("valid");
            assert_eq!(mv.to_string(), text);
        }
        let promotion: Move = "e7e8r".parse().expect("valid");
        assert_eq!(promotion.promotion, Some(PieceKind::Rook));
    }

    #[test]
    fn broken_moves_are_refused() {
        assert!(matches!(
            "e2e9".parse::<Move>(),
            Err(ParseMoveError::Square(_))
        ));
        assert_eq!("e2e4x".parse::<Move>(), Err(ParseMoveError::Promotion('x')));
        assert_eq!("e2".parse::<Move>(), Err(ParseMoveError::Length));
    }
}
