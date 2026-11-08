/// Cores per review engine: a quarter of the computer at most goes to a review.
const CORES_EACH: usize = 4;
/// Never more engines than this, however many cores there are.
const MOST_ENGINES: usize = 3;

/// How many engines, of one thread each, review a game side by side: a quarter of the cores, at
/// least one and at most three, so a review never weighs on the computer.
pub fn review_engines(cores: usize) -> usize {
    (cores / CORES_EACH).clamp(1, MOST_ENGINES)
}

#[cfg(test)]
mod tests {
    use super::review_engines;

    #[test]
    fn a_review_takes_a_quarter_of_the_cores_from_one_to_three() {
        assert_eq!(review_engines(2), 1);
        assert_eq!(review_engines(4), 1);
        assert_eq!(review_engines(8), 2);
        assert_eq!(review_engines(18), 3);
        assert_eq!(review_engines(64), 3);
    }
}
