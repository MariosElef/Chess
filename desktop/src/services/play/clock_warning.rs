use std::time::Duration;

use chess_domain::Side;

/// The warning sounds once a clock is down to this share of its starting time: one part in
/// twenty, the last 5%.
const LOW_TIME_PARTS: u32 = 20;

/// Which players have already been warned that their clock is low; each hears it once a game.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ClockWarning {
    white_warned: bool,
    black_warned: bool,
}

impl ClockWarning {
    /// Whether `side`, with `left` on a clock that started at `initial`, should hear the
    /// warning now: the first time its time is down to the last 5%.
    pub fn is_due(&mut self, side: Side, left: Duration, initial: Duration) -> bool {
        let warned = self.warned_mut(side);
        if *warned || left > low_time(initial) {
            return false;
        }
        *warned = true;
        true
    }

    fn warned_mut(&mut self, side: Side) -> &mut bool {
        match side {
            Side::White => &mut self.white_warned,
            Side::Black => &mut self.black_warned,
        }
    }
}

/// The last 5% of `initial`: 9 seconds of 3 minutes, 30 of 10.
fn low_time(initial: Duration) -> Duration {
    initial / LOW_TIME_PARTS
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use chess_domain::Side;

    use super::{ClockWarning, low_time};

    fn seconds(count: u64) -> Duration {
        Duration::from_secs(count)
    }

    #[test]
    fn low_time_is_the_last_five_percent() {
        assert_eq!(low_time(seconds(180)), seconds(9));
        assert_eq!(low_time(seconds(600)), seconds(30));
        assert_eq!(low_time(seconds(60)), seconds(3));
    }

    #[test]
    fn each_player_is_warned_once_in_their_last_five_percent() {
        let mut warning = ClockWarning::default();
        let three_minutes = seconds(180);
        assert!(!warning.is_due(Side::White, seconds(10), three_minutes));
        assert!(warning.is_due(Side::White, seconds(9), three_minutes));
        assert!(!warning.is_due(Side::White, seconds(4), three_minutes));
        assert!(warning.is_due(Side::Black, seconds(2), three_minutes));
    }
}
