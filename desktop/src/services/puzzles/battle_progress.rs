//! Where the player is in a Puzzle Battle, and how it is said.
use std::time::Duration;

use super::strikes::STRIKES;
use crate::{
    enums::puzzles::{BattleResult, BattleStatus},
    models::puzzles::BattleSnapshot,
};

/// The puzzle the player is on after `answered` answers and `mistakes`; `None` once out.
pub fn current_puzzle(answered: u32, mistakes: u32, total: usize) -> Option<usize> {
    let index = usize::try_from(answered).unwrap_or(usize::MAX);
    (mistakes < STRIKES && index < total).then_some(index)
}

/// The battle's time left, `since` the snapshot arrived.
pub fn battle_left(battle: &BattleSnapshot, since: Duration) -> Duration {
    match battle.status {
        BattleStatus::Finished => Duration::ZERO,
        BattleStatus::Ongoing => Duration::from_millis(battle.left_ms).saturating_sub(since),
    }
}

/// The battle's result as its panel says it: "You won!", "You lost" or "A draw".
pub fn result_words(result: BattleResult) -> &'static str {
    match result {
        BattleResult::Won => "You won!",
        BattleResult::Lost => "You lost",
        BattleResult::Drawn => "A draw",
    }
}

/// `Battle rating +12`, after a rated battle.
pub fn rating_change_text(change: Option<i32>) -> Option<String> {
    change.map(|change| format!("Battle rating {change:+}"))
}

#[cfg(test)]
mod tests {
    use super::{current_puzzle, rating_change_text};

    #[test]
    fn three_mistakes_or_the_last_puzzle_end_a_run() {
        assert_eq!(current_puzzle(4, 1, 40), Some(4));
        assert_eq!(current_puzzle(5, 3, 40), None);
        assert_eq!(current_puzzle(40, 0, 40), None);
        assert_eq!(
            rating_change_text(Some(12)).as_deref(),
            Some("Battle rating +12")
        );
    }
}
