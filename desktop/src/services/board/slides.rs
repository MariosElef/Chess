//! Which pieces slide when the board goes from one shown position to the next.
use chess_domain::{Move, PieceKind, Position, Square};

/// The pieces to slide, each from its old square to its new one, when the board moves one move
/// forward or one move back; none for a jump or a new position. Castling slides the rook too.
pub fn move_slides(
    (before, before_last): (&Position, Option<Move>),
    (after, after_last): (&Position, Option<Move>),
) -> Vec<(Square, Square)> {
    if let Some(mv) = after_last
        && leads_to(before, mv, after)
    {
        return slides_of(before, mv);
    }
    if let Some(mv) = before_last
        && leads_to(after, mv, before)
    {
        return slides_of(after, mv)
            .into_iter()
            .map(|(from, to)| (to, from))
            .collect();
    }
    Vec::new()
}

fn leads_to(from: &Position, mv: Move, to: &Position) -> bool {
    from.play(mv)
        .is_ok_and(|played| played.is_same_position(to))
}

/// The move's piece, and the rook when it castles.
fn slides_of(position: &Position, mv: Move) -> Vec<(Square, Square)> {
    let mut slides = vec![(mv.from, mv.to)];
    let castles = position
        .board()
        .piece_at(mv.from)
        .is_some_and(|piece| piece.kind == PieceKind::King)
        && mv.from.file().abs_diff(mv.to.file()) == 2;
    if castles && let Some(rook) = castling_rook(mv) {
        slides.push(rook);
    }
    slides
}

/// The rook's way in a castle: from the corner to beside the king.
fn castling_rook(king: Move) -> Option<(Square, Square)> {
    let rank = king.from.rank();
    let (corner, beside) = if king.to.file() > king.from.file() {
        (7, 5)
    } else {
        (0, 3)
    };
    Some((Square::new(corner, rank)?, Square::new(beside, rank)?))
}

#[cfg(test)]
mod tests {
    use chess_domain::{Move, Position, Square};

    use super::move_slides;

    fn mv(uci: &str) -> Move {
        uci.parse().expect("a move")
    }

    fn sq(name: &str) -> Square {
        name.parse().expect("a square")
    }

    #[test]
    fn a_step_forward_slides_the_piece_and_a_step_back_slides_it_home() {
        let start = Position::starting();
        let after = start.play(mv("e2e4")).expect("legal");
        let forward = move_slides((&start, None), (&after, Some(mv("e2e4"))));
        assert_eq!(forward, [(sq("e2"), sq("e4"))]);
        let back = move_slides((&after, Some(mv("e2e4"))), (&start, None));
        assert_eq!(back, [(sq("e4"), sq("e2"))]);
    }

    #[test]
    fn castling_slides_the_rook_too_and_a_jump_slides_nothing() {
        let before: Position = "r3k2r/8/8/8/8/8/8/R3K2R w KQkq - 0 1"
            .parse()
            .expect("a FEN");
        let castled = before.play(mv("e1g1")).expect("legal");
        let slides = move_slides((&before, None), (&castled, Some(mv("e1g1"))));
        assert_eq!(slides, [(sq("e1"), sq("g1")), (sq("h1"), sq("f1"))]);
        let far = castled.play(mv("e8c8")).expect("legal");
        assert!(move_slides((&before, None), (&far, Some(mv("e8c8")))).is_empty());
    }
}
