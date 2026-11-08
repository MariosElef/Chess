//! Time controls and the two players' clocks. Time is passed in, never read here, so the
//! desktop's ticking clock and the server's authoritative one share the same rules.
use std::time::Duration;

use crate::{Category, Side};

/// Starting time per player, and the increment after each of their moves: 3 | 2 is 3 min + 2 s.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TimeControl {
    pub initial: Duration,
    pub increment: Duration,
}

impl TimeControl {
    pub const fn new(minutes: u64, increment_seconds: u64) -> Self {
        Self {
            initial: Duration::from_secs(minutes * 60),
            increment: Duration::from_secs(increment_seconds),
        }
    }

    /// Lichess's rule: a game is expected to last about initial + 40 × increment.
    pub fn category(self) -> Category {
        match self.estimated_seconds() {
            0..180 => Category::Bullet,
            180..480 => Category::Blitz,
            480..1500 => Category::Rapid,
            _ => Category::Classical,
        }
    }

    /// `3 min`, or `3 | 2` with an increment.
    pub fn label(self) -> String {
        let minutes = self.initial.as_secs() / 60;
        let increment = self.increment.as_secs();
        if increment == 0 {
            format!("{minutes} min")
        } else {
            format!("{minutes} | {increment}")
        }
    }

    fn estimated_seconds(self) -> u64 {
        self.initial.as_secs() + 40 * self.increment.as_secs()
    }
}

/// Both players' time left at the start of the current turn.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Clock {
    control: TimeControl,
    white: Duration,
    black: Duration,
}

impl Clock {
    pub fn new(control: TimeControl) -> Self {
        Self {
            control,
            white: control.initial,
            black: control.initial,
        }
    }

    pub fn control(&self) -> TimeControl {
        self.control
    }

    /// Time left for `side`, `elapsed` into its turn (zero when it is not that side's turn).
    pub fn remaining(&self, side: Side, elapsed: Duration) -> Duration {
        self.banked(side).saturating_sub(elapsed)
    }

    /// The side ran out of time during its turn.
    pub fn is_flagged(&self, side: Side, elapsed: Duration) -> bool {
        self.remaining(side, elapsed).is_zero()
    }

    /// `side` moved after thinking for `elapsed`: its time is spent, then the increment is added.
    pub fn finish_turn(&mut self, side: Side, elapsed: Duration) {
        self.spend(side, elapsed);
        let with_increment = self.banked(side) + self.control.increment;
        self.set_banked(side, with_increment);
    }

    /// Takes `elapsed` off `side`'s time without an increment, as when the game ends mid-turn.
    pub fn spend(&mut self, side: Side, elapsed: Duration) {
        let left = self.remaining(side, elapsed);
        self.set_banked(side, left);
    }

    fn set_banked(&mut self, side: Side, time: Duration) {
        match side {
            Side::White => self.white = time,
            Side::Black => self.black = time,
        }
    }

    fn banked(&self, side: Side) -> Duration {
        match side {
            Side::White => self.white,
            Side::Black => self.black,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::{Clock, TimeControl};
    use crate::{Category, Side};

    #[test]
    fn categories_follow_the_estimated_length() {
        assert_eq!(TimeControl::new(1, 0).category(), Category::Bullet);
        assert_eq!(TimeControl::new(2, 1).category(), Category::Bullet);
        assert_eq!(TimeControl::new(3, 0).category(), Category::Blitz);
        assert_eq!(TimeControl::new(5, 3).category(), Category::Blitz);
        assert_eq!(TimeControl::new(10, 0).category(), Category::Rapid);
        assert_eq!(TimeControl::new(15, 10).category(), Category::Rapid);
        assert_eq!(TimeControl::new(30, 0).category(), Category::Classical);
    }

    #[test]
    fn labels_show_minutes_and_increment() {
        assert_eq!(TimeControl::new(10, 0).label(), "10 min");
        assert_eq!(TimeControl::new(3, 2).label(), "3 | 2");
    }

    #[test]
    fn a_move_spends_the_time_used_and_adds_the_increment() {
        let mut clock = Clock::new(TimeControl::new(3, 2));
        clock.finish_turn(Side::White, Duration::from_secs(10));
        assert_eq!(
            clock.remaining(Side::White, Duration::ZERO),
            Duration::from_secs(172)
        );
        assert_eq!(
            clock.remaining(Side::Black, Duration::ZERO),
            Duration::from_secs(180)
        );
    }

    #[test]
    fn spending_time_adds_no_increment() {
        let mut clock = Clock::new(TimeControl::new(3, 2));
        clock.spend(Side::White, Duration::from_secs(10));
        assert_eq!(
            clock.remaining(Side::White, Duration::ZERO),
            Duration::from_secs(170)
        );
    }

    #[test]
    fn the_clock_runs_down_during_a_turn_and_flags_at_zero() {
        let clock = Clock::new(TimeControl::new(1, 0));
        assert_eq!(
            clock.remaining(Side::Black, Duration::from_secs(45)),
            Duration::from_secs(15)
        );
        assert!(!clock.is_flagged(Side::Black, Duration::from_secs(59)));
        assert!(clock.is_flagged(Side::Black, Duration::from_secs(61)));
    }
}
