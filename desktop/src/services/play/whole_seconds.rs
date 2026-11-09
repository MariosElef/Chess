use std::time::Duration;

/// A duration in whole seconds, as the live messages count clocks.
pub fn whole_seconds(duration: Duration) -> u32 {
    u32::try_from(duration.as_secs()).unwrap_or(u32::MAX)
}
