//! Lichess's accuracy: how much of the winning chances each move kept.
#![expect(
    clippy::float_arithmetic,
    reason = "accuracy is an exponential curve over winning chances"
)]

/// A move that loses nothing is 100; one that throws away all of a winning position is 0.
pub fn move_accuracy(expected_points_lost: f64) -> f64 {
    let win_percent_lost = expected_points_lost * 100.0;
    (103.166_8 * (-0.043_54 * win_percent_lost).exp() - 3.166_9).clamp(0.0, 100.0)
}

/// The mean of a side's move accuracies; a side that has not moved is 100.
pub fn side_accuracy(moves: &[f64]) -> f64 {
    if moves.is_empty() {
        return 100.0;
    }
    let count = u32::try_from(moves.len()).unwrap_or(u32::MAX);
    moves.iter().sum::<f64>() / f64::from(count)
}

#[cfg(test)]
mod tests {
    use super::{move_accuracy, side_accuracy};

    #[test]
    fn a_move_that_loses_nothing_is_perfect_and_a_blunder_is_low() {
        assert!((move_accuracy(0.0) - 100.0).abs() < 0.01);
        assert!(move_accuracy(0.05) > 75.0);
        assert!(move_accuracy(0.3) < 30.0);
        assert!(move_accuracy(1.0).abs() < f64::EPSILON, "never below zero");
    }

    #[test]
    fn a_side_is_the_mean_of_its_moves() {
        assert!((side_accuracy(&[100.0, 50.0]) - 75.0).abs() < 1e-9);
        assert!((side_accuracy(&[]) - 100.0).abs() < 1e-9);
    }
}
