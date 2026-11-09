use std::time::Duration;

/// What is left of `wait` once `elapsed` has gone, from 1 down to 0, as a toast's bar shows it.
#[expect(
    clippy::float_arithmetic,
    reason = "a share of the wait, drawn as a bar's width, is a fraction"
)]
pub fn time_left(elapsed: Duration, wait: Duration) -> f32 {
    if wait.is_zero() {
        return 0.;
    }
    (1. - elapsed.as_secs_f32() / wait.as_secs_f32()).clamp(0., 1.)
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::time_left;

    #[test]
    fn the_bar_shrinks_from_full_to_empty() {
        let wait = Duration::from_secs(60);
        assert!((time_left(Duration::ZERO, wait) - 1.).abs() < f32::EPSILON);
        assert!((time_left(Duration::from_secs(30), wait) - 0.5).abs() < 1e-6);
        assert!(time_left(Duration::from_secs(90), wait).abs() < f32::EPSILON);
        assert!(time_left(Duration::ZERO, Duration::ZERO).abs() < f32::EPSILON);
    }
}
