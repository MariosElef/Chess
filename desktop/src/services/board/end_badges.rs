use chess_domain::{Board, Outcome, Side, Square};

use crate::enums::board::EndBadge;

/// The kings' badges for how the game ended: the winner's crown and the loser's reason, or a draw.
pub fn end_badges(board: &Board, outcome: Outcome) -> Vec<(Square, EndBadge)> {
    let badges = match outcome {
        Outcome::Checkmate { winner } => won(winner, EndBadge::Checkmated(winner.opponent())),
        Outcome::Resignation { winner } => won(winner, EndBadge::Resigned(winner.opponent())),
        Outcome::Timeout { winner } => won(winner, EndBadge::OutOfTime(winner.opponent())),
        Outcome::Stalemate
        | Outcome::DrawAgreement
        | Outcome::InsufficientMaterial
        | Outcome::Repetition
        | Outcome::FiftyMoves => [
            (Side::White, EndBadge::Drawn(Side::White)),
            (Side::Black, EndBadge::Drawn(Side::Black)),
        ],
    };
    badges
        .into_iter()
        .filter_map(|(side, badge)| Some((board.king_of(side)?, badge)))
        .collect()
}

/// The winner's crown and the loser's badge, by side.
fn won(winner: Side, loser: EndBadge) -> [(Side, EndBadge); 2] {
    [(winner, EndBadge::Winner), (winner.opponent(), loser)]
}

#[cfg(test)]
mod tests {
    use chess_domain::{Outcome, Position, Side};

    use super::end_badges;
    use crate::enums::board::EndBadge;

    #[test]
    fn the_winner_wears_the_crown_and_the_loser_the_reason() {
        let position = Position::starting();
        let badges = end_badges(
            position.board(),
            Outcome::Resignation {
                winner: Side::Black,
            },
        );
        let names: Vec<(String, EndBadge)> = badges
            .into_iter()
            .map(|(square, badge)| (square.to_string(), badge))
            .collect();
        assert_eq!(
            names,
            [
                ("e8".to_owned(), EndBadge::Winner),
                ("e1".to_owned(), EndBadge::Resigned(Side::White)),
            ]
        );
    }

    #[test]
    fn a_draw_marks_both_kings() {
        let position = Position::starting();
        let badges = end_badges(position.board(), Outcome::Repetition);
        assert_eq!(badges.len(), 2);
        assert!(
            badges
                .iter()
                .all(|(_, badge)| matches!(badge, EndBadge::Drawn(_)))
        );
    }
}
