//! Picking a premove: like `selection`, but for the side not to move, on the board as the
//! queued premoves leave it, with the loose premove targets instead of the legal moves.
use chess_domain::{Board, Move, PieceKind, Side, Square};

use crate::enums::board::ClickOutcome;

/// Clicking a premove target queues the move; any other click reselects a piece of `side`.
pub fn premove_outcome(
    board: &Board,
    side: Side,
    selected: Option<Square>,
    clicked: Square,
) -> ClickOutcome {
    match selected.filter(|&from| board.premove_targets(from).contains(&clicked)) {
        Some(from) if is_promotion(board, from, clicked) => {
            ClickOutcome::Promote { from, to: clicked }
        }
        Some(from) => ClickOutcome::Play(Move::new(from, clicked)),
        None => ClickOutcome::Select(next_selection(board, side, selected, clicked)),
    }
}

/// The squares the selected piece could premove to.
pub fn premove_targets(board: &Board, selected: Option<Square>) -> Vec<Square> {
    selected.map_or_else(Vec::new, |from| board.premove_targets(from))
}

/// Clicking the selected piece again lets go; another piece of `side` takes its place.
fn next_selection(
    board: &Board,
    side: Side,
    selected: Option<Square>,
    clicked: Square,
) -> Option<Square> {
    if selected == Some(clicked) {
        return None;
    }
    board
        .piece_at(clicked)
        .is_some_and(|piece| piece.side == side)
        .then_some(clicked)
}

/// A pawn reaching the far rank promotes.
fn is_promotion(board: &Board, from: Square, to: Square) -> bool {
    board
        .piece_at(from)
        .is_some_and(|piece| piece.kind == PieceKind::Pawn && (to.rank() == 0 || to.rank() == 7))
}

#[cfg(test)]
mod tests {
    use chess_domain::{Board, Move, Position, Side, Square};

    use super::{ClickOutcome, premove_outcome, premove_targets};

    fn sq(name: &str) -> Square {
        name.parse().expect("a square")
    }

    #[test]
    fn clicking_a_target_queues_the_move_even_through_pieces() {
        let start = Board::starting();
        assert_eq!(
            premove_outcome(&start, Side::Black, Some(sq("f8")), sq("c5")),
            ClickOutcome::Play(Move::new(sq("f8"), sq("c5")))
        );
    }

    #[test]
    fn clicking_elsewhere_changes_the_selection() {
        let start = Board::starting();
        assert_eq!(
            premove_outcome(&start, Side::Black, Some(sq("e7")), sq("e4")),
            ClickOutcome::Select(None)
        );
        assert_eq!(
            premove_outcome(&start, Side::Black, None, sq("g8")),
            ClickOutcome::Select(Some(sq("g8")))
        );
        assert_eq!(
            premove_outcome(&start, Side::Black, None, sq("g1")),
            ClickOutcome::Select(None),
            "the other side's pieces cannot be premoved"
        );
        assert_eq!(
            premove_outcome(&start, Side::Black, Some(sq("g8")), sq("g8")),
            ClickOutcome::Select(None)
        );
    }

    #[test]
    fn a_pawn_reaching_the_last_rank_asks_what_to_become() {
        let board = *"8/8/8/8/8/8/4p3/k6K b - - 0 1"
            .parse::<Position>()
            .expect("valid")
            .board();
        assert_eq!(
            premove_outcome(&board, Side::Black, Some(sq("e2")), sq("e1")),
            ClickOutcome::Promote {
                from: sq("e2"),
                to: sq("e1")
            }
        );
    }

    #[test]
    fn targets_follow_the_selection() {
        let start = Board::starting();
        assert_eq!(premove_targets(&start, Some(sq("g8"))).len(), 3);
        assert!(premove_targets(&start, None).is_empty());
    }
}
