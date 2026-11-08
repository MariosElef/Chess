/// How the latest result moved the rating: the last rating less the one before it.
pub fn rating_change(history: &[i32]) -> Option<i32> {
    match history {
        [.., before, last] => Some(last - before),
        [] | [_] => None,
    }
}

/// The graph's line as chess.com steps it, as fractions of its box: across from 0 to 1, and
/// down from 0 at the top to 1 at the bottom. Each rating holds until the next; a lone rating,
/// or none, is a flat line through the middle.
#[expect(
    clippy::float_arithmetic,
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    reason = "a rating's place in the graph's box, as fractions drawn in f32"
)]
pub fn graph_steps(history: &[i32]) -> Vec<(f32, f32)> {
    let (Some(low), Some(high)) = (history.iter().min(), history.iter().max()) else {
        return flat_line();
    };
    if history.len() < 2 || low == high {
        return flat_line();
    }
    let span = f64::from(high - low);
    let step = 1. / (history.len() - 1) as f64;
    let mut points = Vec::with_capacity(history.len() * 2);
    for (index, rating) in history.iter().enumerate() {
        let across = index as f64 * step;
        // A margin top and bottom, so the line never touches the box's edges.
        let down = 0.15 + 0.7 * (1. - f64::from(rating - low) / span);
        if let Some(&(_, held)) = points.last() {
            points.push((across as f32, held));
        }
        points.push((across as f32, down as f32));
    }
    points
}

fn flat_line() -> Vec<(f32, f32)> {
    vec![(0., 0.5), (1., 0.5)]
}

#[cfg(test)]
mod tests {
    use super::{graph_steps, rating_change};

    #[test]
    fn the_change_is_the_latest_results() {
        assert_eq!(rating_change(&[1500, 1512, 1498]), Some(-14));
        assert_eq!(rating_change(&[1500]), None);
        assert_eq!(rating_change(&[]), None);
    }

    #[test]
    fn the_graph_steps_from_rating_to_rating_with_the_highest_on_top() {
        assert_eq!(graph_steps(&[]), [(0., 0.5), (1., 0.5)]);
        assert_eq!(graph_steps(&[1500, 1500]), [(0., 0.5), (1., 0.5)]);
        let steps = graph_steps(&[1500, 1600]);
        assert_eq!(steps.len(), 3, "the low rating, a step across, then up");
        assert_eq!(steps[0], (0., 0.85));
        assert_eq!(steps[1], (1., 0.85));
        assert!((steps[2].1 - 0.15).abs() < 1e-6, "the highest near the top");
    }
}
