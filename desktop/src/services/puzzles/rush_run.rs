use std::time::Duration;

use super::strikes::STRIKES;

use crate::{enums::puzzles::RushKind, models::puzzles::RatingRange};

/// The first puzzle's rating; each one after it is this much harder.
const FIRST_RATING: i32 = 600;
const RATING_STEP: i32 = 40;
/// How wide a band each puzzle is picked from.
const BAND_WIDTH: i32 = 100;
/// The band asked for instead when its narrow one has no puzzle left that the player has not
/// seen: near the top of the ratings, puzzles are few.
const WIDE_BAND_WIDTH: i32 = 600;

/// One Puzzle Rush: how many puzzles were served, solved and missed. Each puzzle comes from a
/// higher band than the last, so the rush starts easy and climbs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RushRun {
    kind: RushKind,
    served: u32,
    solved: u32,
    strikes: u32,
}

impl RushRun {
    pub fn new(kind: RushKind) -> Self {
        Self {
            kind,
            served: 0,
            solved: 0,
            strikes: 0,
        }
    }

    pub fn kind(&self) -> RushKind {
        self.kind
    }

    /// The score: puzzles solved without a mistake.
    pub fn solved(&self) -> u32 {
        self.solved
    }

    pub fn strikes(&self) -> u32 {
        self.strikes
    }

    /// The band the `index`th puzzle (from 0) is picked from.
    pub fn band(index: u32) -> RatingRange {
        let steps = i32::try_from(index).unwrap_or(i32::MAX / RATING_STEP);
        RatingRange::around(FIRST_RATING + RATING_STEP * steps, BAND_WIDTH)
    }

    /// The band for the puzzle after the ones served so far.
    pub fn next_band(&self) -> RatingRange {
        Self::band(self.served)
    }

    /// The next puzzle's band, widened.
    pub fn next_wide_band(&self) -> RatingRange {
        let narrow = self.next_band();
        RatingRange::around((narrow.min + narrow.max) / 2, WIDE_BAND_WIDTH)
    }

    pub fn served_one(&mut self) {
        self.served += 1;
    }

    pub fn solved_one(&mut self) {
        self.solved += 1;
    }

    /// A wrong move: a strike, and that puzzle is lost.
    pub fn missed_one(&mut self) {
        self.strikes += 1;
    }

    /// Time left after `elapsed`, or `None` in survival.
    pub fn time_left(&self, elapsed: Duration) -> Option<Duration> {
        self.kind.limit().map(|limit| limit.saturating_sub(elapsed))
    }

    /// Three strikes, or the clock ran out.
    pub fn is_over(&self, elapsed: Duration) -> bool {
        self.strikes >= STRIKES || self.time_left(elapsed).is_some_and(|left| left.is_zero())
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::RushRun;
    use crate::{enums::puzzles::RushKind, models::puzzles::RatingRange};

    #[test]
    fn the_puzzles_start_easy_and_climb() {
        assert_eq!(RushRun::band(0), RatingRange { min: 550, max: 650 });
        assert_eq!(
            RushRun::band(10),
            RatingRange {
                min: 950,
                max: 1050
            }
        );
        assert!(RushRun::band(30).min > RushRun::band(29).min);
    }

    #[test]
    fn the_wide_band_holds_the_narrow_one() {
        let rush = RushRun::new(RushKind::Survival);
        let (narrow, wide) = (rush.next_band(), rush.next_wide_band());
        assert!(wide.min <= narrow.min && wide.max >= narrow.max);
        assert!(wide.max - wide.min > narrow.max - narrow.min);
    }

    #[test]
    fn three_strikes_end_it() {
        let mut rush = RushRun::new(RushKind::Survival);
        rush.missed_one();
        rush.missed_one();
        assert!(
            !rush.is_over(Duration::from_secs(3600)),
            "survival has no clock"
        );
        rush.missed_one();
        assert!(rush.is_over(Duration::ZERO));
    }

    #[test]
    fn the_clock_ends_a_timed_rush() {
        let rush = RushRun::new(RushKind::ThreeMinutes);
        assert_eq!(
            rush.time_left(Duration::from_secs(60)),
            Some(Duration::from_secs(120))
        );
        assert!(!rush.is_over(Duration::from_secs(179)));
        assert!(rush.is_over(Duration::from_secs(180)));
    }

    #[test]
    fn each_puzzle_served_moves_to_the_next_band() {
        let mut rush = RushRun::new(RushKind::FiveMinutes);
        let first = rush.next_band();
        rush.served_one();
        rush.solved_one();
        assert!(rush.next_band().min > first.min);
        assert_eq!(rush.solved(), 1);
    }
}
