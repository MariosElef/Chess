//! What a move is called, from the facts the engine gives about it. The order of the checks is
//! the rule: the first that fits names the move.
#![expect(
    clippy::float_arithmetic,
    reason = "the classes are bands of expected points lost"
)]

use crate::enums::analysis::MoveClass;

/// The upper limits of expected points lost, from chess.com's published table.
const EXCELLENT: f64 = 0.02;
const GOOD: f64 = 0.05;
const INACCURACY: f64 = 0.10;
const MISTAKE: f64 = 0.20;
/// Not in a bad position after a Brilliant move.
const NOT_BAD: f64 = 0.45;
/// Winning, for Miss.
const WINNING: f64 = 0.75;
/// Completely winning: a Brilliant move must not be needed to stay there.
const COMPLETELY_WINNING: f64 = 0.9;

/// What the engine says about one move, all from the mover's point of view.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MoveFacts {
    /// Legal moves in the position it was played from.
    pub legal_moves: usize,
    /// Still following the opening table.
    pub book: bool,
    /// The engine's top move.
    pub played_best: bool,
    /// Expected points with the engine's best move, and with its second best, if it gave one.
    pub best_points: f64,
    pub second_points: Option<f64>,
    /// Expected points after the move played.
    pub played_points: f64,
    /// It gives up a piece the opponent can take.
    pub sacrifice: bool,
    /// The opponent's move just before was a Mistake or worse.
    pub after_opponent_error: bool,
}

impl MoveFacts {
    /// Expected points lost against the best move; nothing for the best move itself.
    pub fn loss(&self) -> f64 {
        if self.played_best {
            0.0
        } else {
            (self.best_points - self.played_points).max(0.0)
        }
    }

    /// Whether the move is bad enough that the next one may punish it.
    pub fn is_error(&self) -> bool {
        self.loss() > INACCURACY
    }
}

pub fn classify(facts: &MoveFacts) -> MoveClass {
    if facts.legal_moves == 1 {
        return MoveClass::Forced;
    }
    if facts.book {
        return MoveClass::Book;
    }
    if is_brilliant(facts) {
        return MoveClass::Brilliant;
    }
    if is_great(facts) {
        return MoveClass::Great;
    }
    if facts.played_best {
        return MoveClass::Best;
    }
    if is_miss(facts) {
        return MoveClass::Miss;
    }
    band(facts.loss())
}

/// A good piece sacrifice: nearly best, not worse off, and not from an already won position.
fn is_brilliant(facts: &MoveFacts) -> bool {
    let winning_without = facts.second_points.unwrap_or(facts.best_points) >= COMPLETELY_WINNING;
    facts.sacrifice
        && facts.loss() <= EXCELLENT
        && facts.played_points >= NOT_BAD
        && !winning_without
}

/// The best move when it was the only good one: anything else gave up a tenth of a point.
fn is_great(facts: &MoveFacts) -> bool {
    facts.played_best
        && facts
            .second_points
            .is_some_and(|second| facts.best_points - second >= INACCURACY)
}

/// The opponent erred and left a win on the board, and this move let it go.
fn is_miss(facts: &MoveFacts) -> bool {
    facts.after_opponent_error
        && facts.best_points >= WINNING
        && facts.played_points < WINNING
        && facts.loss() > GOOD
}

/// Chess.com's bands of expected points lost.
fn band(loss: f64) -> MoveClass {
    if loss <= EXCELLENT {
        MoveClass::Excellent
    } else if loss <= GOOD {
        MoveClass::Good
    } else if loss <= INACCURACY {
        MoveClass::Inaccuracy
    } else if loss <= MISTAKE {
        MoveClass::Mistake
    } else {
        MoveClass::Blunder
    }
}

#[cfg(test)]
mod tests {
    use super::{MoveFacts, classify};
    use crate::enums::analysis::MoveClass;

    /// An ordinary move in an even position that keeps what the best move would.
    fn quiet(played_points: f64) -> MoveFacts {
        MoveFacts {
            legal_moves: 30,
            book: false,
            played_best: false,
            best_points: 0.5,
            second_points: Some(0.49),
            played_points,
            sacrifice: false,
            after_opponent_error: false,
        }
    }

    #[test]
    fn the_bands_follow_the_points_lost() {
        assert_eq!(classify(&quiet(0.49)), MoveClass::Excellent);
        assert_eq!(classify(&quiet(0.46)), MoveClass::Good);
        assert_eq!(classify(&quiet(0.42)), MoveClass::Inaccuracy);
        assert_eq!(classify(&quiet(0.35)), MoveClass::Mistake);
        assert_eq!(classify(&quiet(0.1)), MoveClass::Blunder);
    }

    #[test]
    fn forced_book_and_best_come_first() {
        let only_move = MoveFacts {
            legal_moves: 1,
            ..quiet(0.1)
        };
        assert_eq!(classify(&only_move), MoveClass::Forced);
        let book = MoveFacts {
            book: true,
            ..quiet(0.42)
        };
        assert_eq!(classify(&book), MoveClass::Book);
        let best = MoveFacts {
            played_best: true,
            ..quiet(0.48)
        };
        assert_eq!(classify(&best), MoveClass::Best);
    }

    #[test]
    fn a_sound_sacrifice_is_brilliant_unless_already_winning() {
        let sacrifice = MoveFacts {
            played_best: true,
            sacrifice: true,
            ..quiet(0.6)
        };
        assert_eq!(classify(&sacrifice), MoveClass::Brilliant);
        let already_won = MoveFacts {
            best_points: 0.97,
            second_points: Some(0.95),
            ..sacrifice
        };
        assert_ne!(classify(&already_won), MoveClass::Brilliant);
        let unsound = MoveFacts {
            played_best: false,
            ..sacrifice
        };
        let unsound = MoveFacts {
            played_points: 0.2,
            ..unsound
        };
        assert_eq!(classify(&unsound), MoveClass::Blunder);
    }

    #[test]
    fn the_only_good_move_is_great() {
        let only_good = MoveFacts {
            played_best: true,
            second_points: Some(0.3),
            ..quiet(0.5)
        };
        assert_eq!(classify(&only_good), MoveClass::Great);
        let one_of_many = MoveFacts {
            second_points: Some(0.48),
            ..only_good
        };
        assert_eq!(classify(&one_of_many), MoveClass::Best);
    }

    #[test]
    fn letting_the_opponents_error_go_is_a_miss() {
        let missed = MoveFacts {
            best_points: 0.85,
            second_points: Some(0.84),
            after_opponent_error: true,
            ..quiet(0.55)
        };
        assert_eq!(classify(&missed), MoveClass::Miss);
        let no_error_before = MoveFacts {
            after_opponent_error: false,
            ..missed
        };
        assert_eq!(classify(&no_error_before), MoveClass::Blunder);
    }
}
