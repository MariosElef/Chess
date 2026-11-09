use std::time::{Duration, Instant};

use super::time_left;

/// How often a countdown's bar moves: often enough to look smooth, rarely enough to stay cheap.
pub const COUNTDOWN_TICK: Duration = Duration::from_millis(250);

/// A wait the server started, counted here from how much of it the server said was left.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Countdown {
    /// When the wait began, as near as the time said to be left lets this app tell.
    pub began: Instant,
    pub wait: Duration,
}

impl Countdown {
    /// The wait of `wait_s` seconds with `seconds_left` of it to go.
    pub fn from_server(wait_s: u32, seconds_left: u32) -> Self {
        let wait = Duration::from_secs(wait_s.into());
        let gone = wait.saturating_sub(Duration::from_secs(seconds_left.into()));
        Self {
            began: Instant::now()
                .checked_sub(gone)
                .unwrap_or_else(Instant::now),
            wait,
        }
    }

    pub fn is_over(&self) -> bool {
        self.began.elapsed() >= self.wait
    }

    /// What is left of the wait, from 1 down to 0, as a toast's bar shows it.
    pub fn left(&self) -> f32 {
        time_left(self.began.elapsed(), self.wait)
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::Countdown;

    #[test]
    fn a_wait_is_counted_from_the_time_the_server_said_was_left() {
        let countdown = Countdown::from_server(60, 20);
        let gone = countdown.began.elapsed();
        assert!(gone >= Duration::from_secs(40), "{gone:?}");
        assert!(gone < Duration::from_secs(41), "{gone:?}");
        assert!(!countdown.is_over());
        assert!(Countdown::from_server(60, 0).is_over());
    }
}
