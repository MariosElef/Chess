//! What the end-of-game card says.
use chess_domain::{Outcome, Side};

use crate::services::board::side_name;

/// "You Beat Danis!", "Danis Won", "You Won!", "You Lost", "White Won" or "Draw": `me` is
/// the player's side, `None` for two players at one computer; `opponent` names a bot.
pub fn result_title(winner: Option<Side>, me: Option<Side>, opponent: Option<&str>) -> String {
    match (winner, me) {
        (None, _) => "Draw".to_owned(),
        (Some(winner), None) => format!("{} Won", side_name(winner)),
        (Some(winner), Some(me)) if winner == me => {
            opponent.map_or_else(|| "You Won!".to_owned(), |name| format!("You Beat {name}!"))
        }
        (Some(_), Some(_)) => {
            opponent.map_or_else(|| "You Lost".to_owned(), |name| format!("{name} Won"))
        }
    }
}

/// How it ended, under the card's title: "by checkmate", "by time", "by the fifty-move rule",
/// as an online game's panel says it.
pub fn outcome_reason(outcome: Outcome) -> &'static str {
    match outcome {
        Outcome::Checkmate { .. } => "by checkmate",
        Outcome::Resignation { .. } => "by resignation",
        Outcome::Timeout { .. } => "by time",
        Outcome::Stalemate => "by stalemate",
        Outcome::DrawAgreement => "by agreement",
        Outcome::InsufficientMaterial => "by insufficient material",
        Outcome::Repetition => "by threefold repetition",
        Outcome::FiftyMoves => "by the fifty-move rule",
    }
}

#[cfg(test)]
mod tests {
    use chess_domain::{Outcome, Side};

    use super::{outcome_reason, result_title};

    #[test]
    fn the_title_is_said_to_whoever_is_looking() {
        let white = Some(Side::White);
        assert_eq!(result_title(white, white, Some("Danis")), "You Beat Danis!");
        assert_eq!(
            result_title(white, Some(Side::Black), Some("Danis")),
            "Danis Won"
        );
        assert_eq!(result_title(white, Some(Side::Black), None), "You Lost");
        assert_eq!(result_title(white, None, None), "White Won");
        assert_eq!(result_title(None, white, None), "Draw");
        let mate = Outcome::Checkmate {
            winner: Side::White,
        };
        assert_eq!(outcome_reason(mate), "by checkmate");
    }
}
