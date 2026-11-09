//! One made-up game between two players of known strength: real legal moves, clocks that run,
//! and a result the stronger side tends to win. Randomness comes in as a source of rolls from 0
//! to 1, so a test can give fixed ones.
#![expect(
    clippy::float_arithmetic,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss,
    reason = "made-up games are chance and proportions"
)]

use chess_domain::{GameStatus, Position, Side};
use chrono::{DateTime, Duration, Utc};
use uuid::Uuid;

use crate::{
    enums::play::{Color, GameResult},
    models::seed::{FakeGame, FakeMove},
};

/// A category's clock: its name, starting seconds and increment.
pub type Terms = (&'static str, i32, i32);

/// The categories and how often each is played, most often first.
pub const CATEGORY_WEIGHTS: [(Terms, f64); 4] = [
    (("blitz", 180, 2), 0.40),
    (("bullet", 60, 0), 0.25),
    (("rapid", 600, 0), 0.25),
    (("classical", 1800, 0), 0.10),
];

/// The fewest and most plies a game runs before someone gives up, unless it is mated first.
const PLIES: (u32, u32) = (24, 110);
/// How often a game that is not mated ends in a draw by agreement, and how often in a timeout.
const DRAW_CHANCE: f64 = 0.08;
const TIMEOUT_CHANCE: f64 = 0.25;

/// The two players, White first, with their strengths.
pub struct Pairing {
    pub white: (Uuid, f64),
    pub black: (Uuid, f64),
}

/// The game `pairing` plays under `terms`, starting at `started_at`.
pub fn play_game(
    pairing: &Pairing,
    terms: Terms,
    started_at: DateTime<Utc>,
    roll: &mut impl FnMut() -> f64,
) -> FakeGame {
    let (category, initial_s, increment_s) = terms;
    let plies = PLIES.0 + (roll() * f64::from(PLIES.1 - PLIES.0)) as u32;
    let mut position = Position::starting();
    let mut clocks = (initial_s * 1000, initial_s * 1000);
    let mut at = started_at;
    let mut moves = Vec::new();
    while moves.len() < plies as usize && position.status() == GameStatus::Ongoing {
        let legal = position.legal_moves();
        let mv = legal[index_for(roll(), legal.len())];
        let mover = position.side_to_move();
        let spent = think_ms(initial_s, roll());
        clocks = after_move(clocks, mover, spent, increment_s);
        at += Duration::milliseconds(i64::from(spent));
        position = position
            .play(mv)
            .expect("the move is one of the legal ones");
        moves.push(FakeMove {
            uci: mv.to_string(),
            white_ms: clocks.0,
            black_ms: clocks.1,
            played_at: at,
        });
    }
    let (result, winner) = ending(&position, pairing, roll);
    if result == GameResult::Timeout {
        clocks = flagged(clocks, winner);
    }
    FakeGame {
        id: Uuid::now_v7(),
        white_id: pairing.white.0,
        black_id: pairing.black.0,
        category,
        initial_s,
        increment_s,
        fen: position.to_string(),
        moves,
        result,
        winner,
        white_ms: clocks.0,
        black_ms: clocks.1,
        started_at,
        finished_at: at + Duration::seconds(2),
    }
}

/// How it ended: the board's own ending if it has one, else a draw, a resignation or a timeout,
/// the stronger side winning more often.
fn ending(
    position: &Position,
    pairing: &Pairing,
    roll: &mut impl FnMut() -> f64,
) -> (GameResult, Option<Color>) {
    match position.status() {
        GameStatus::Checkmate { winner } => (GameResult::Checkmate, Some(Color::from(winner))),
        GameStatus::Stalemate => (GameResult::Stalemate, None),
        GameStatus::Ongoing => decided(pairing, roll),
    }
}

fn decided(pairing: &Pairing, roll: &mut impl FnMut() -> f64) -> (GameResult, Option<Color>) {
    if roll() < DRAW_CHANCE {
        return (GameResult::DrawAgreement, None);
    }
    let winner = if roll() < white_wins(pairing) {
        Color::White
    } else {
        Color::Black
    };
    let result = if roll() < TIMEOUT_CHANCE {
        GameResult::Timeout
    } else {
        GameResult::Resignation
    };
    (result, Some(winner))
}

/// White's chance against Black, by the Elo curve.
fn white_wins(pairing: &Pairing) -> f64 {
    1.0 / (1.0 + 10f64.powf((pairing.black.1 - pairing.white.1) / 400.0))
}

/// A move's thinking time: up to a thirtieth of the starting clock.
fn think_ms(initial_s: i32, roll: f64) -> i32 {
    let most = f64::from(initial_s) * 1000.0 / 30.0;
    (200.0 + roll * most) as i32
}

/// Both clocks after `mover` spends `spent` and gets the increment, never below a tenth of a second.
fn after_move((white, black): (i32, i32), mover: Side, spent: i32, increment_s: i32) -> (i32, i32) {
    let charge = |clock: i32| (clock - spent + increment_s * 1000).max(100);
    match mover {
        Side::White => (charge(white), black),
        Side::Black => (white, charge(black)),
    }
}

/// The loser of a timeout has nothing left on their clock.
fn flagged((white, black): (i32, i32), winner: Option<Color>) -> (i32, i32) {
    match winner {
        Some(Color::White) => (white, 0),
        Some(Color::Black) => (0, black),
        None => (white, black),
    }
}

/// A roll from 0 to 1 as an index into `len` things.
pub fn index_for(roll: f64, len: usize) -> usize {
    ((roll.clamp(0.0, 0.999_999) * len as f64) as usize).min(len.saturating_sub(1))
}

#[cfg(test)]
mod tests {
    use chess_domain::Position;
    use chrono::Utc;
    use uuid::Uuid;

    use super::{Pairing, play_game};
    use crate::enums::play::GameResult;

    /// Rolls that cycle through a fixed list, so a game is the same every run.
    fn rolls(values: &'static [f64]) -> impl FnMut() -> f64 {
        let mut next = 0;
        move || {
            let value = values[next % values.len()];
            next += 1;
            value
        }
    }

    #[test]
    fn a_game_is_legal_moves_with_running_clocks_and_an_ending() {
        let pairing = Pairing {
            white: (Uuid::now_v7(), 1500.0),
            black: (Uuid::now_v7(), 1500.0),
        };
        let mut roll = rolls(&[0.31, 0.77, 0.05, 0.62, 0.48, 0.9, 0.13]);
        let game = play_game(&pairing, ("blitz", 180, 2), Utc::now(), &mut roll);
        assert!(!game.moves.is_empty());
        // Replaying the moves from the start reaches the stored position.
        let mut position = Position::starting();
        for mv in &game.moves {
            position = position
                .play(mv.uci.parse().expect("a move"))
                .expect("legal");
        }
        assert_eq!(position.to_string(), game.fen);
        assert!(
            game.moves
                .iter()
                .all(|mv| mv.white_ms > 0 || mv.black_ms > 0)
        );
        assert!(game.finished_at > game.started_at);
        assert!(GameResult::ALL.contains(&game.result));
    }
}
