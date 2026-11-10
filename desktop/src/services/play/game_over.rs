//! What the end-of-game card says.
use chess_domain::{Outcome, Side};

use super::side_label;
use crate::i18n::tr;

/// The card's title, as "You Won!" or "Draw": `me` is the player's side, `opponent` a bot's name.
pub fn result_title(winner: Option<Side>, me: Option<Side>, opponent: Option<&str>) -> String {
    match (winner, me) {
        (None, _) => tr!("play.title.draw"),
        (Some(winner), None) => tr!("play.title.side_won", side = side_label(winner)),
        (Some(winner), Some(me)) if winner == me => opponent.map_or_else(
            || tr!("play.title.you_won"),
            |name| tr!("play.title.you_beat", name = name),
        ),
        (Some(_), Some(_)) => opponent.map_or_else(
            || tr!("play.title.you_lost"),
            |name| tr!("play.title.bot_won", name = name),
        ),
    }
}

/// How it ended, under the card's title: "by checkmate", "by time", "by the fifty-move rule".
pub fn outcome_reason(outcome: Outcome) -> String {
    let ending = match outcome {
        Outcome::Checkmate { .. } => tr!("play.ending.checkmate"),
        Outcome::Resignation { .. } => tr!("play.ending.resignation"),
        Outcome::Timeout { .. } => tr!("play.ending.timeout"),
        Outcome::Stalemate => tr!("play.ending.stalemate"),
        Outcome::DrawAgreement => tr!("play.ending.draw_agreement"),
        Outcome::InsufficientMaterial => tr!("play.ending.insufficient_material"),
        Outcome::Repetition => tr!("play.ending.repetition"),
        Outcome::FiftyMoves => tr!("play.ending.fifty_move"),
    };
    tr!("play.reason.by", reason = ending)
}

#[cfg(test)]
mod tests {
    use chess_domain::{Outcome, Side};

    use super::{outcome_reason, result_title};
    use crate::{enums::settings::Language, i18n::set_current};

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

    #[test]
    fn the_title_and_reason_follow_the_language() {
        set_current(Language::Greek);
        assert_eq!(result_title(None, None, None), "Ισοπαλία");
        let mate = Outcome::Checkmate {
            winner: Side::White,
        };
        assert_eq!(outcome_reason(mate), "με ματ");
        set_current(Language::English);
    }
}
