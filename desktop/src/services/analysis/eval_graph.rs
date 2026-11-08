//! The review's graph of the evaluation, as fractions of its width and height.

/// Each position's point on the top edge of White's area, from start to end, share 1 at top.
pub fn graph_points(shares: &[f64]) -> Vec<(f32, f32)> {
    let last = shares.len().saturating_sub(1).max(1);
    shares
        .iter()
        .enumerate()
        .map(|(index, &share)| (fraction(index, last), height_of(share)))
        .collect()
}

/// The position nearest `across`, a fraction of the graph's width, among `count`.
pub fn position_at(across: f32, count: usize) -> usize {
    let last = count.saturating_sub(1);
    #[expect(
        clippy::float_arithmetic,
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        clippy::cast_precision_loss,
        reason = "a fraction of the width rounded to a position, clamped between the ends"
    )]
    let nearest = (across.clamp(0., 1.) * last as f32).round() as usize;
    nearest.min(last)
}

#[expect(
    clippy::float_arithmetic,
    clippy::cast_precision_loss,
    reason = "a position's place along the graph, as a fraction"
)]
fn fraction(index: usize, last: usize) -> f32 {
    index as f32 / last as f32
}

/// How far down White's area reaches: none at a share of 1, all of it at 0.
#[expect(
    clippy::float_arithmetic,
    clippy::cast_possible_truncation,
    reason = "a share between 0 and 1, as a fraction of the height"
)]
fn height_of(share: f64) -> f32 {
    (1. - share.clamp(0., 1.)) as f32
}

#[cfg(test)]
mod tests {
    use super::{graph_points, position_at};

    #[test]
    fn the_graph_runs_from_the_start_to_the_end() {
        let points = graph_points(&[0.5, 1.0, 0.0]);
        assert_eq!(points, [(0.0, 0.5), (0.5, 0.0), (1.0, 1.0)]);
        assert_eq!(position_at(0.0, 3), 0);
        assert_eq!(position_at(0.6, 3), 1);
        assert_eq!(position_at(2.0, 3), 2);
    }
}
