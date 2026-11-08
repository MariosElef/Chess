//! An online game as the server sends it, read for the board and the panel.
use std::time::Duration;

use chess_domain::{Game, Move, Outcome, Side, TimeControl};

use crate::{
    enums::play::{GameResult, LiveStatus},
    models::play::{Challenge, GameSnapshot},
    services::board::side_name,
};

/// The game the snapshot's moves make, played from the start; `None` if a move does not parse
/// or is not legal, which the server never sends.
pub fn replay(snapshot: &GameSnapshot) -> Option<Game> {
    let mut game = Game::new();
    for uci in &snapshot.moves {
        let mv: Move = uci.parse().ok()?;
        game.play(mv).ok()?;
    }
    Some(game)
}

/// The side the player with `player_id` has in the game, if they are in it.
pub fn my_side(snapshot: &GameSnapshot, player_id: &str) -> Option<Side> {
    if snapshot.white.id == player_id {
        Some(Side::White)
    } else if snapshot.black.id == player_id {
        Some(Side::Black)
    } else {
        None
    }
}

/// `side`'s time left, `since` the snapshot arrived: the running clock keeps counting down.
pub fn clock_left(snapshot: &GameSnapshot, side: Side, since: Duration) -> Duration {
    let sent = Duration::from_millis(match side {
        Side::White => snapshot.white_ms,
        Side::Black => snapshot.black_ms,
    });
    let running = snapshot.running.map(Side::from) == Some(side);
    if running {
        sent.saturating_sub(since)
    } else {
        sent
    }
}

/// How the game ended, said to `me`: "You won by checkmate", "Draw by agreement", "Game
/// aborted"; `None` while it is going on.
pub fn result_line(snapshot: &GameSnapshot, me: Option<Side>) -> Option<String> {
    match snapshot.status {
        LiveStatus::Ongoing => return None,
        LiveStatus::Aborted => return Some("Game aborted".to_owned()),
        LiveStatus::Finished => {}
    }
    let by = snapshot.result.map_or("", |result| result.by());
    let line = match (snapshot.winner.map(Side::from), me) {
        (None, _) => format!("Draw by {by}"),
        (Some(winner), Some(me)) if winner == me => format!("You won by {by}"),
        (Some(_), Some(_)) => format!("You lost by {by}"),
        (Some(winner), None) => format!("{} won by {by}", side_name(winner)),
    };
    Some(line)
}

/// What a challenge asks for: `5 | 3 · rated`, `3 min · casual`.
pub fn challenge_terms(challenge: &Challenge) -> String {
    let control = TimeControl {
        initial: Duration::from_secs(challenge.initial_s.into()),
        increment: Duration::from_secs(challenge.increment_s.into()),
    };
    let kind = if challenge.rated { "rated" } else { "casual" };
    format!("{} · {kind}", control.label())
}

/// How a finished game ended, for the kings' badges; `None` while it goes on or once aborted.
pub fn outcome_of(snapshot: &GameSnapshot) -> Option<Outcome> {
    if snapshot.status != LiveStatus::Finished {
        return None;
    }
    let winner = snapshot.winner.map(Side::from);
    let outcome = match (snapshot.result?, winner) {
        (GameResult::Checkmate, Some(winner)) => Outcome::Checkmate { winner },
        (GameResult::Resignation, Some(winner)) => Outcome::Resignation { winner },
        (GameResult::Timeout, Some(winner)) => Outcome::Timeout { winner },
        (GameResult::Checkmate | GameResult::Resignation | GameResult::Timeout, None) => {
            return None;
        }
        (GameResult::Stalemate, _) => Outcome::Stalemate,
        (GameResult::DrawAgreement, _) => Outcome::DrawAgreement,
        (GameResult::InsufficientMaterial, _) => Outcome::InsufficientMaterial,
        (GameResult::Repetition, _) => Outcome::Repetition,
        (GameResult::FiftyMove, _) => Outcome::FiftyMoves,
    };
    Some(outcome)
}

/// The game can still be called off: neither side has made its second move.
pub fn can_abort(snapshot: &GameSnapshot) -> bool {
    snapshot.status == LiveStatus::Ongoing && snapshot.moves.len() < 2
}

/// `Rating 1508 (+8)` for `me` after a rated game.
pub fn rating_line(snapshot: &GameSnapshot, me: Side) -> Option<String> {
    let changes = snapshot.rating_changes?;
    let (player, change) = match me {
        Side::White => (&snapshot.white, changes.white),
        Side::Black => (&snapshot.black, changes.black),
    };
    let before = player.rating?;
    Some(format!("Rating {} ({change:+})", before + change))
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use chess_domain::Side;

    use super::{
        can_abort, challenge_terms, clock_left, my_side, outcome_of, rating_line, replay,
        result_line,
    };
    use crate::{
        enums::play::{Color, GameResult, LiveStatus},
        models::{
            play::{Challenge, GameSnapshot, PlayerAtBoard, RatingChanges},
            social::UserRef,
        },
    };

    fn snapshot() -> GameSnapshot {
        GameSnapshot {
            id: "g".to_owned(),
            white: PlayerAtBoard {
                id: "ann".to_owned(),
                username: "Ann".to_owned(),
                rating: Some(1500),
            },
            black: PlayerAtBoard {
                id: "bob".to_owned(),
                username: "Bob".to_owned(),
                rating: Some(1500),
            },
            rated: true,
            initial_s: 180,
            increment_s: 2,
            moves: vec!["e2e4".to_owned(), "e7e5".to_owned()],
            white_ms: 175_000,
            black_ms: 170_000,
            running: Some(Color::White),
            status: LiveStatus::Ongoing,
            result: None,
            winner: None,
            draw_offer: None,
            rating_changes: None,
            chat_id: "chat".to_owned(),
        }
    }

    #[test]
    fn the_moves_replay_into_the_game() {
        let game = replay(&snapshot()).expect("legal moves");
        assert_eq!(game.sans(), ["e4", "e5"]);
        let broken = GameSnapshot {
            moves: vec!["e2e5".to_owned()],
            ..snapshot()
        };
        assert!(replay(&broken).is_none());
    }

    #[test]
    fn only_the_running_clock_counts_down() {
        let since = Duration::from_secs(5);
        assert_eq!(
            clock_left(&snapshot(), Side::White, since),
            Duration::from_secs(170)
        );
        assert_eq!(
            clock_left(&snapshot(), Side::Black, since),
            Duration::from_secs(170)
        );
        assert_eq!(
            clock_left(&snapshot(), Side::White, Duration::from_secs(500)),
            Duration::ZERO
        );
    }

    #[test]
    fn the_ending_is_said_to_each_player() {
        assert_eq!(my_side(&snapshot(), "bob"), Some(Side::Black));
        assert_eq!(result_line(&snapshot(), Some(Side::Black)), None);
        let mated = GameSnapshot {
            status: LiveStatus::Finished,
            result: Some(GameResult::Checkmate),
            winner: Some(Color::Black),
            rating_changes: Some(RatingChanges {
                white: -8,
                black: 8,
            }),
            ..snapshot()
        };
        assert_eq!(
            result_line(&mated, Some(Side::Black)).as_deref(),
            Some("You won by checkmate")
        );
        assert_eq!(
            result_line(&mated, Some(Side::White)).as_deref(),
            Some("You lost by checkmate")
        );
        assert_eq!(
            rating_line(&mated, Side::Black).as_deref(),
            Some("Rating 1508 (+8)")
        );
        assert_eq!(
            outcome_of(&mated),
            Some(chess_domain::Outcome::Checkmate {
                winner: Side::Black
            })
        );
        assert_eq!(outcome_of(&snapshot()), None, "still going");
        let drawn = GameSnapshot {
            status: LiveStatus::Finished,
            result: Some(GameResult::DrawAgreement),
            ..snapshot()
        };
        assert_eq!(
            result_line(&drawn, None).as_deref(),
            Some("Draw by agreement")
        );
        let aborted = GameSnapshot {
            status: LiveStatus::Aborted,
            ..snapshot()
        };
        assert_eq!(
            result_line(&aborted, Some(Side::White)).as_deref(),
            Some("Game aborted")
        );
        assert!(!can_abort(&aborted));
        assert!(!can_abort(&snapshot()), "both sides have moved");
        let one_move = GameSnapshot {
            moves: vec!["e2e4".to_owned()],
            ..snapshot()
        };
        assert!(can_abort(&one_move));
    }

    #[test]
    fn a_challenge_says_its_clock_and_whether_it_counts() {
        let person = |name: &str| UserRef {
            id: name.to_owned(),
            username: name.to_owned(),
        };
        let challenge = Challenge {
            id: "c".to_owned(),
            from: person("ann"),
            to: person("bob"),
            initial_s: 300,
            increment_s: 3,
            rated: true,
        };
        assert_eq!(challenge_terms(&challenge), "5 | 3 · rated");
        let casual = Challenge {
            increment_s: 0,
            rated: false,
            ..challenge
        };
        assert_eq!(challenge_terms(&casual), "5 min · casual");
    }
}
