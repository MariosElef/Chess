use std::time::{Duration, Instant};

use crate::enums::history::StepPace;

/// Steps further apart than this are separate presses; closer, a key or button being held. A
/// held key repeats every tenth of a second or so, quicker than anyone taps.
const HELD_GAP: Duration = Duration::from_millis(150);
/// Held this long, the walk goes fast.
const FAST_AFTER: Duration = Duration::from_secs(2);

/// Steps that keep coming close together, as from a held key or button: when the run began
/// and when it last stepped.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct StepStreak {
    began: Option<Instant>,
    last: Option<Instant>,
}

impl StepStreak {
    /// Notes a step at `now`: fast once steps have kept coming for two seconds.
    pub fn step(&mut self, now: Instant) -> StepPace {
        let held = self
            .last
            .is_some_and(|last| now.saturating_duration_since(last) <= HELD_GAP);
        if !held {
            self.began = Some(now);
        }
        self.last = Some(now);
        let began = self.began.unwrap_or(now);
        if now.saturating_duration_since(began) >= FAST_AFTER {
            StepPace::Fast
        } else {
            StepPace::Normal
        }
    }
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, Instant};

    use super::StepStreak;
    use crate::enums::history::StepPace;

    #[test]
    fn steps_held_for_two_seconds_go_fast_and_a_pause_slows_them() {
        let start = Instant::now();
        let at = |millis: u64| start + Duration::from_millis(millis);
        let mut streak = StepStreak::default();
        let mut pace = StepPace::Normal;
        for millis in (0..=2_100).step_by(100) {
            pace = streak.step(at(millis));
            if millis < 2_000 {
                assert_eq!(pace, StepPace::Normal, "at {millis} ms");
            }
        }
        assert_eq!(pace, StepPace::Fast);
        assert_eq!(
            streak.step(at(3_000)),
            StepPace::Normal,
            "let go, then pressed again"
        );
    }

    #[test]
    fn quick_taps_are_not_a_held_key() {
        let start = Instant::now();
        let mut streak = StepStreak::default();
        for millis in (0..=3_000).step_by(200) {
            let pace = streak.step(start + Duration::from_millis(millis));
            assert_eq!(
                pace,
                StepPace::Normal,
                "a tap every fifth of a second, at {millis} ms"
            );
        }
    }
}
