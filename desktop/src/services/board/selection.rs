use chess_domain::{Move, Position, Square};

use crate::enums::board::ClickOutcome;

/// Clicking a marked square plays the move or asks for a promotion; others change the selection.
pub fn click_outcome(
    position: &Position,
    selected: Option<Square>,
    clicked: Square,
) -> ClickOutcome {
    let moves = moves_to(position, selected, clicked);
    match moves.first() {
        Some(mv) if mv.promotion.is_some() => ClickOutcome::Promote {
            from: mv.from,
            to: mv.to,
        },
        Some(&mv) => ClickOutcome::Play(mv),
        None => ClickOutcome::Select(next_selection(position, selected, clicked)),
    }
}

/// The legal moves from the selected square to `clicked`: one, or four for a promotion.
fn moves_to(position: &Position, selected: Option<Square>, clicked: Square) -> Vec<Move> {
    let Some(from) = selected else {
        return Vec::new();
    };
    position
        .legal_moves_from(from)
        .into_iter()
        .filter(|mv| mv.to == clicked)
        .collect()
}

/// What is selected after clicking `clicked`: a piece of the side to move, or nothing.
pub fn next_selection(
    position: &Position,
    selected: Option<Square>,
    clicked: Square,
) -> Option<Square> {
    if selected == Some(clicked) {
        return None;
    }
    is_own_piece(position, clicked).then_some(clicked)
}

/// The squares the selected piece can move to; a promotion's four choices count once.
pub fn targets(position: &Position, selected: Option<Square>) -> Vec<Square> {
    let Some(from) = selected else {
        return Vec::new();
    };
    let mut squares: Vec<Square> = position
        .legal_moves_from(from)
        .iter()
        .map(|mv| mv.to)
        .collect();
    squares.sort_unstable();
    squares.dedup();
    squares
}

fn is_own_piece(position: &Position, square: Square) -> bool {
    position
        .board()
        .piece_at(square)
        .is_some_and(|piece| piece.side == position.side_to_move())
}

#[cfg(test)]
mod tests {
    use chess_domain::{Move, Position, Square};

    use super::{ClickOutcome, click_outcome, next_selection, targets};

    #[test]
    fn clicking_a_marked_square_plays_the_move() {
        let start = Position::starting();
        assert_eq!(
            click_outcome(&start, Some(sq("e2")), sq("e4")),
            ClickOutcome::Play(Move::new(sq("e2"), sq("e4")))
        );
    }

    #[test]
    fn clicking_an_unmarked_square_only_changes_the_selection() {
        let start = Position::starting();
        assert_eq!(
            click_outcome(&start, Some(sq("e2")), sq("e5")),
            ClickOutcome::Select(None)
        );
        assert_eq!(
            click_outcome(&start, None, sq("g1")),
            ClickOutcome::Select(Some(sq("g1")))
        );
    }

    #[test]
    fn the_king_shows_and_plays_both_castles() {
        let position: Position = "r3k2r/8/8/8/8/8/8/R3K2R w KQkq - 0 1"
            .parse()
            .expect("valid");
        let king_targets = targets(&position, Some(sq("e1")));
        assert!(king_targets.contains(&sq("g1")));
        assert!(king_targets.contains(&sq("c1")));
        assert_eq!(
            click_outcome(&position, Some(sq("e1")), sq("g1")),
            ClickOutcome::Play(Move::new(sq("e1"), sq("g1")))
        );
    }

    #[test]
    fn a_pawn_reaching_the_last_rank_asks_what_to_become() {
        let position: Position = "8/4P3/8/8/8/8/8/k6K w - - 0 1".parse().expect("valid");
        assert_eq!(
            click_outcome(&position, Some(sq("e7")), sq("e8")),
            ClickOutcome::Promote {
                from: sq("e7"),
                to: sq("e8")
            }
        );
    }

    fn sq(name: &str) -> Square {
        name.parse().expect("a square")
    }

    #[test]
    fn clicking_your_piece_selects_it() {
        let start = Position::starting();
        assert_eq!(next_selection(&start, None, sq("e2")), Some(sq("e2")));
    }

    #[test]
    fn clicking_it_again_lets_go() {
        let start = Position::starting();
        assert_eq!(next_selection(&start, Some(sq("e2")), sq("e2")), None);
    }

    #[test]
    fn clicking_another_piece_of_yours_switches_to_it() {
        let start = Position::starting();
        assert_eq!(
            next_selection(&start, Some(sq("e2")), sq("g1")),
            Some(sq("g1"))
        );
    }

    #[test]
    fn empty_squares_and_enemy_pieces_clear_the_selection() {
        let start = Position::starting();
        assert_eq!(next_selection(&start, Some(sq("e2")), sq("e4")), None);
        assert_eq!(next_selection(&start, None, sq("e7")), None);
    }

    #[test]
    fn a_pawn_on_its_start_square_has_two_targets() {
        let start = Position::starting();
        assert_eq!(targets(&start, Some(sq("e2"))), [sq("e3"), sq("e4")]);
        assert!(targets(&start, None).is_empty());
    }

    #[test]
    fn a_promotion_shows_one_target() {
        let position: Position = "8/4P3/8/8/8/8/8/k6K w - - 0 1".parse().expect("valid");
        assert_eq!(targets(&position, Some(sq("e7"))), [sq("e8")]);
    }
}
