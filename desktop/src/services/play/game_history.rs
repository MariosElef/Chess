use std::time::Duration;

use chess_domain::{Game, Outcome, Side, TimeControl};

use super::online_game::{game_from_uci, outcome_from};
use crate::{
    enums::play::PlayerResult,
    models::play::{GameRecord, PlayedGame},
};

/// "Won", "Lost" or "Draw", as the history's chip says it.
pub fn result_word(result: PlayerResult) -> &'static str {
    match result {
        PlayerResult::Won => "Won",
        PlayerResult::Lost => "Lost",
        PlayerResult::Drawn => "Draw",
    }
}

/// "3 | 2 · Rated · 24 moves": the time control, whether it counted, and its length.
pub fn game_line(game: &PlayedGame) -> String {
    let control = TimeControl {
        initial: Duration::from_secs(game.initial_s.into()),
        increment: Duration::from_secs(game.increment_s.into()),
    }
    .label();
    let rated = if game.rated { "Rated" } else { "Casual" };
    let moves = game.plies.div_ceil(2);
    let plural = if moves == 1 { "" } else { "s" };
    format!("{control} · {rated} · {moves} move{plural}")
}

/// The game rebuilt from its moves, and how it ended; `None` if a stored move will not play.
pub fn opened_game(record: &GameRecord) -> Option<(Game, Option<Outcome>)> {
    let game = game_from_uci(&record.moves)?;
    let mine = Side::from(record.game.color);
    let winner = match record.game.result {
        PlayerResult::Won => Some(mine),
        PlayerResult::Lost => Some(mine.opponent()),
        PlayerResult::Drawn => None,
    };
    Some((game, outcome_from(record.game.outcome, winner)))
}

#[cfg(test)]
mod tests {
    use chess_domain::{Outcome, Side};

    use super::{game_line, opened_game};
    use crate::{
        enums::play::{Color, GameResult, PlayerResult},
        models::play::{GameRecord, PlayedGame},
    };

    fn played(result: PlayerResult) -> PlayedGame {
        PlayedGame {
            id: "g1".to_owned(),
            color: Color::White,
            opponent: "bob".to_owned(),
            result,
            outcome: GameResult::Checkmate,
            category: "blitz".to_owned(),
            initial_s: 180,
            increment_s: 2,
            rated: true,
            plies: 4,
            ago_s: 60,
            rating: Some(1490),
            rating_change: Some(-10),
        }
    }

    #[test]
    fn a_game_is_summed_up_in_one_line() {
        assert_eq!(
            game_line(&played(PlayerResult::Lost)),
            "3 | 2 · Rated · 2 moves"
        );
    }

    #[test]
    fn a_lost_game_reopens_with_the_other_side_winning() {
        let record = GameRecord {
            game: played(PlayerResult::Lost),
            moves: ["f2f3", "e7e5", "g2g4", "d8h4"].map(str::to_owned).to_vec(),
        };
        let (game, outcome) = opened_game(&record).expect("the moves play");
        assert_eq!(game.moves().len(), 4);
        assert_eq!(
            outcome,
            Some(Outcome::Checkmate {
                winner: Side::Black
            })
        );
    }
}
