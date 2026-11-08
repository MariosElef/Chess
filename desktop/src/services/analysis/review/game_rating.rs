//! The rating a side played like in one game, from its accuracy: chess.com's Game Rating. Their
//! mapping is not published; this one follows the average accuracy players reach at each rating
//! over many games, so it is an estimate, and one game moves it a lot.
#![expect(
    clippy::float_arithmetic,
    reason = "the estimate is read off a curve between known points"
)]

/// Average accuracy and the rating it is typical of, accuracy rising.
const CURVE: [(f64, f64); 10] = [
    (30.0, 100.0),
    (50.0, 400.0),
    (60.0, 800.0),
    (68.0, 1200.0),
    (75.0, 1500.0),
    (80.0, 1750.0),
    (85.0, 2000.0),
    (90.0, 2400.0),
    (95.0, 2800.0),
    (100.0, 3200.0),
];
/// Estimates are given to the nearest fifty: anything finer would claim more than one game says.
const STEP: f64 = 50.0;
/// Fewer moves of a side's own than this, and there is too little to go on.
pub const FEWEST_MOVES: usize = 5;

/// The rating `accuracy` is typical of, to the nearest fifty.
pub fn estimated_rating(accuracy: f64) -> u32 {
    let rating = along_curve(accuracy.clamp(CURVE[0].0, 100.0));
    // The curve keeps it between 100 and 3200, so it fits.
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "100..=3200"
    )]
    let rounded = ((rating / STEP).round() * STEP) as u32;
    rounded
}

/// Straight lines between the curve's points.
fn along_curve(accuracy: f64) -> f64 {
    let (mut low, mut high) = (CURVE[0], CURVE[CURVE.len() - 1]);
    for pair in CURVE.windows(2) {
        if accuracy <= pair[1].0 {
            (low, high) = (pair[0], pair[1]);
            break;
        }
    }
    let share = (accuracy - low.0) / (high.0 - low.0);
    low.1 + share * (high.1 - low.1)
}

#[cfg(test)]
mod tests {
    use super::estimated_rating;

    #[test]
    fn higher_accuracy_reads_as_a_higher_rating() {
        assert_eq!(estimated_rating(75.0), 1500);
        assert_eq!(estimated_rating(85.0), 2000);
        assert_eq!(
            estimated_rating(77.5),
            1650,
            "1625, halfway to 1750, rounds up to fifty"
        );
        assert_eq!(estimated_rating(10.0), 100, "never below the curve");
        assert_eq!(estimated_rating(100.0), 3200);
        assert!(estimated_rating(62.0) < estimated_rating(63.0));
    }
}
