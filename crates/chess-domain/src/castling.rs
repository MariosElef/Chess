use crate::{Side, Square};

/// Which castles are still allowed: lost for good once the king or that rook moves or is taken.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct CastlingRights {
    pub white_king_side: bool,
    pub white_queen_side: bool,
    pub black_king_side: bool,
    pub black_queen_side: bool,
}

impl CastlingRights {
    pub const ALL: Self = Self {
        white_king_side: true,
        white_queen_side: true,
        black_king_side: true,
        black_queen_side: true,
    };

    pub const fn king_side(self, side: Side) -> bool {
        match side {
            Side::White => self.white_king_side,
            Side::Black => self.black_king_side,
        }
    }

    pub const fn queen_side(self, side: Side) -> bool {
        match side {
            Side::White => self.white_queen_side,
            Side::Black => self.black_queen_side,
        }
    }

    pub(crate) fn revoke_side(&mut self, side: Side) {
        match side {
            Side::White => {
                self.white_king_side = false;
                self.white_queen_side = false;
            }
            Side::Black => {
                self.black_king_side = false;
                self.black_queen_side = false;
            }
        }
    }

    /// A move from or to a rook's home corner ends castling on that side.
    pub(crate) fn revoke_corner(&mut self, square: Square) {
        match (square.file(), square.rank()) {
            (7, 0) => self.white_king_side = false,
            (0, 0) => self.white_queen_side = false,
            (7, 7) => self.black_king_side = false,
            (0, 7) => self.black_queen_side = false,
            _ => {}
        }
    }
}

/// The castling field of a FEN: `KQkq` for every right, `-` for none.
impl std::fmt::Display for CastlingRights {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let letters: String = [
            (self.white_king_side, 'K'),
            (self.white_queen_side, 'Q'),
            (self.black_king_side, 'k'),
            (self.black_queen_side, 'q'),
        ]
        .into_iter()
        .filter_map(|(allowed, letter)| allowed.then_some(letter))
        .collect();
        if letters.is_empty() {
            f.write_str("-")
        } else {
            f.write_str(&letters)
        }
    }
}
