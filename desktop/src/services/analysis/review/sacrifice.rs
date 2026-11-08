//! Whether a move gives up a piece: the test behind a Brilliant move.
use chess_domain::{Move, PieceKind, Position, Side, Square, piece_value};

/// A piece worth less than this is not sacrificed: a pawn is not, a knight is.
const LEAST_PIECE: i32 = 3;
/// The material the opponent must be able to win, net, for it to be a sacrifice: a bishop for
/// a pawn is, a bishop for a knight is not.
const LEAST_GIVEN: i32 = 2;

/// The move leaves a piece of the mover's that the opponent can take and come out at least two
/// pawns up on, counting what the move itself captured. A piece that could already be won
/// before the move does not count: the move did not give it up.
pub fn is_sacrifice(before: &Position, mv: Move) -> bool {
    let Ok(after) = before.play(mv) else {
        return false;
    };
    let mover = before.side_to_move();
    let taken = before
        .board()
        .piece_at(mv.to)
        .map_or(0, |piece| piece_value(piece.kind));
    let already_hanging = hanging_squares(&with_turn_passed(before), mover);
    pieces_of(&after, mover).any(|square| {
        let moved_or_new = square == mv.to || !already_hanging.contains(&square);
        moved_or_new && winnable(&after, square).is_some_and(|won| won - taken >= LEAST_GIVEN)
    })
}

/// The squares of `side`'s pieces worth sacrificing that the side to move can win now.
fn hanging_squares(position: &Option<Position>, side: Side) -> Vec<Square> {
    let Some(position) = position else {
        return Vec::new();
    };
    pieces_of(position, side)
        .filter(|&square| winnable(position, square).is_some_and(|won| won >= LEAST_GIVEN))
        .collect()
}

/// The squares of `side`'s pieces worth a sacrifice: not pawns, not the king.
fn pieces_of(position: &Position, side: Side) -> impl Iterator<Item = Square> + '_ {
    Square::all().filter(move |&square| {
        position.board().piece_at(square).is_some_and(|piece| {
            piece.side == side
                && piece.kind != PieceKind::King
                && piece_value(piece.kind) >= LEAST_PIECE
        })
    })
}

/// What the side to move wins by taking on `square` with its cheapest piece: the piece there,
/// less the taker if it can be taken back. `None` when it cannot be taken.
fn winnable(position: &Position, square: Square) -> Option<i32> {
    let victim = piece_value(position.board().piece_at(square)?.kind);
    let capture = position
        .legal_moves()
        .into_iter()
        .filter(|mv| mv.to == square)
        .min_by_key(|mv| taker_value(position, *mv))?;
    let taker = taker_value(position, capture);
    let after = position.play(capture).ok()?;
    let recaptured = after.legal_moves().iter().any(|mv| mv.to == square);
    Some(if recaptured { victim - taker } else { victim })
}

fn taker_value(position: &Position, capture: Move) -> i32 {
    position
        .board()
        .piece_at(capture.from)
        .map_or(0, |piece| piece_value(piece.kind))
}

/// The same position with the other side to move, as if the mover passed; `None` when that is
/// not a legal position (the mover is in check).
fn with_turn_passed(position: &Position) -> Option<Position> {
    if position.is_check() {
        return None;
    }
    let fen = position.to_string();
    let mut fields: Vec<&str> = fen.split_whitespace().collect();
    let passed = match fields.get(1) {
        Some(&"w") => "b",
        Some(_) => "w",
        None => return None,
    };
    fields[1] = passed;
    if let Some(en_passant) = fields.get_mut(3) {
        *en_passant = "-";
    }
    fields.join(" ").parse().ok()
}

#[cfg(test)]
mod tests {
    use chess_domain::{Move, Position};

    use super::is_sacrifice;

    fn position(fen: &str) -> Position {
        fen.parse().expect("a FEN")
    }

    fn mv(uci: &str) -> Move {
        uci.parse().expect("a move")
    }

    #[test]
    fn the_greek_gift_gives_up_a_bishop() {
        let before = position("r1bq1rk1/pppn1ppp/4p3/3pP3/1b1P4/2NB1N2/PPP2PPP/R2QK2R w KQ - 0 8");
        assert!(is_sacrifice(&before, mv("d3h7")));
    }

    #[test]
    fn an_even_trade_is_not_a_sacrifice() {
        let before = position("rnbqkb1r/pppp1ppp/5n2/4p3/4P3/2N5/PPPP1PPP/R1BQKBNR w KQkq - 2 3");
        assert!(!is_sacrifice(&before, mv("f1c4")), "a bishop that is safe");
        let trade = position("r1bqkbnr/pppp1ppp/2n5/1B2p3/4P3/5N2/PPPP1PPP/RNBQK2R w KQkq - 2 3");
        assert!(!is_sacrifice(&trade, mv("b5c6")), "bishop for knight");
    }

    #[test]
    fn a_piece_left_hanging_before_the_move_was_not_given_up_by_it() {
        let before = position("rnbqkbnr/pppp1ppp/8/4p2Q/8/4P3/PPPP1PPP/RNB1KBNR w KQkq - 0 3");
        assert!(
            !is_sacrifice(&before, mv("a2a3")),
            "the queen was already en prise"
        );
    }

    #[test]
    fn moving_a_piece_where_a_pawn_takes_it_is() {
        let before = position("rnbqkbnr/pppp1ppp/8/4p3/4P3/5N2/PPPP1PPP/RNBQKB1R w KQkq - 0 2");
        assert!(is_sacrifice(&before, mv("f3d4")) || is_sacrifice(&before, mv("f1a6")));
    }
}
