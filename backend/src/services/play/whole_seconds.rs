use std::time::Duration;

/// A duration in whole seconds, as the live messages count clocks and waits.
pub fn whole_seconds(duration: Duration) -> u32 {
    u32::try_from(duration.as_secs()).unwrap_or(u32::MAX)
}

/// A duration in whole seconds rounded up, so a wait just begun reads in full.
pub fn whole_seconds_up(duration: Duration) -> u32 {
    u32::try_from(duration.as_millis().div_ceil(1000)).unwrap_or(u32::MAX)
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::{whole_seconds, whole_seconds_up};

    #[test]
    fn seconds_round_down_or_up_as_asked() {
        let nearly_a_minute = Duration::from_millis(59_001);
        assert_eq!(whole_seconds(nearly_a_minute), 59);
        assert_eq!(whole_seconds_up(nearly_a_minute), 60);
        assert_eq!(whole_seconds_up(Duration::from_secs(60)), 60);
        assert_eq!(whole_seconds_up(Duration::ZERO), 0);
    }
}
