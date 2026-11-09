//! A king dropped on its own rook castles that way, as on chess.com: the drop counts as one on
//! the square the king lands on when castling.
use chess_domain::{Board, PieceKind, Square};

/// The file the king lands on when castling short, and long.
const SHORT_CASTLE_FILE: u8 = 6;
const LONG_CASTLE_FILE: u8 = 2;

/// Where a piece dropped from `from` on `to` is meant to go: the castling square when a king is
/// dropped on a rook of its own on its rank, otherwise `to` itself.
pub fn drop_square(board: &Board, from: Square, to: Square) -> Square {
    castling_square(board, from, to).unwrap_or(to)
}

/// The square the king lands on castling toward the rook on `to`, if `from` holds a king and
/// `to` a rook of the same side on the same rank.
fn castling_square(board: &Board, from: Square, to: Square) -> Option<Square> {
    let king = board
        .piece_at(from)
        .filter(|piece| piece.kind == PieceKind::King)?;
    let rook = board
        .piece_at(to)
        .filter(|piece| piece.kind == PieceKind::Rook)?;
    if rook.side != king.side || to.rank() != from.rank() {
        return None;
    }
    let file = if to.file() > from.file() {
        SHORT_CASTLE_FILE
    } else {
        LONG_CASTLE_FILE
    };
    Square::new(file, from.rank())
}

#[cfg(test)]
mod tests {
    use chess_domain::{Position, Square};

    use super::drop_square;

    fn sq(name: &str) -> Square {
        name.parse().expect("a square")
    }

    #[test]
    fn a_king_dropped_on_its_rook_lands_on_the_castling_square() {
        let position: Position = "r3k2r/8/8/8/8/8/8/R3K2R w KQkq - 0 1"
            .parse()
            .expect("valid");
        let board = position.board();
        assert_eq!(drop_square(board, sq("e1"), sq("h1")), sq("g1"));
        assert_eq!(drop_square(board, sq("e1"), sq("a1")), sq("c1"));
        assert_eq!(drop_square(board, sq("e8"), sq("h8")), sq("g8"));
    }

    #[test]
    fn any_other_drop_stays_where_it_was_dropped() {
        let position: Position = "r3k2r/8/8/8/8/8/8/R3K2R w KQkq - 0 1"
            .parse()
            .expect("valid");
        let board = position.board();
        assert_eq!(
            drop_square(board, sq("e1"), sq("h8")),
            sq("h8"),
            "their rook"
        );
        assert_eq!(
            drop_square(board, sq("a1"), sq("h1")),
            sq("h1"),
            "not a king"
        );
        assert_eq!(drop_square(board, sq("e1"), sq("f1")), sq("f1"));
    }
}
