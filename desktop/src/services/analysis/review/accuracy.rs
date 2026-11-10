//! Lichess's game accuracy, exactly as Lichess computes it: every move's accuracy from the
//! winning chances it kept, then for each side the mean of a volatility-weighted mean and a
//! harmonic mean, so one blunder weighs as much as it hurt. It follows lila's
//! `AccuracyPercent.scala`, scalachess's `WinPercent` and scalalib's `Maths`, and matches the
//! accuracy Lichess shows for its own games.
#![expect(
    clippy::float_arithmetic,
    reason = "accuracy is an exponential curve over winning chances"
)]

use chess_domain::{Side, engine::Eval};

/// Evaluations past ten pawns, and mates, count as ten pawns.
const CEILING: i32 = 1000;
/// Lichess's fit from centipawns to winning chances.
const WIN_CURVE: f64 = 0.003_682_08;
/// The sliding window over positions is a tenth of the game, between these.
const WINDOW: (usize, usize) = (2, 8);
/// A window's volatility, the deviation of its winning chances, is kept between these.
const WEIGHT: (f64, f64) = (0.5, 12.0);

/// White's chances of winning from an evaluation, from 0 to 100.
pub fn win_percent(eval: Eval) -> f64 {
    let centipawns = match eval {
        Eval::Centipawns(centipawns) => centipawns.clamp(-CEILING, CEILING),
        Eval::Mate {
            winner: Side::White,
            ..
        } => CEILING,
        Eval::Mate {
            winner: Side::Black,
            ..
        } => -CEILING,
    };
    let chances = 2.0 / (1.0 + (-WIN_CURVE * f64::from(centipawns)).exp()) - 1.0;
    50.0 + 50.0 * chances.clamp(-1.0, 1.0)
}

/// One move's accuracy from the mover's winning chances before and after it: 100 for a move that
/// loses nothing, falling steeply with what it gave away. The 1 is Lichess's allowance for the
/// engine's own imprecision.
fn move_accuracy(before: f64, after: f64) -> f64 {
    if after >= before {
        return 100.0;
    }
    let lost = before - after;
    let raw =
        103.166_810_071_164_9 * (-0.043_544_153_867_539_51 * lost).exp() - 3.166_924_740_191_411;
    (raw + 1.0).clamp(0.0, 100.0)
}

/// White's then Black's accuracy over a game, from the evaluation of every position, the one
/// before the first move included; `first_mover` made the first move. A side with no moves has
/// none.
pub fn game_accuracy(evals: &[Eval], first_mover: Side) -> [Option<f64>; 2] {
    let wins: Vec<f64> = evals.iter().copied().map(win_percent).collect();
    let weights = volatility_weights(&wins);
    let mut by_side: [Vec<(f64, f64)>; 2] = [Vec::new(), Vec::new()];
    for (index, (pair, weight)) in wins.windows(2).zip(weights).enumerate() {
        let mover = if index % 2 == 0 {
            first_mover
        } else {
            first_mover.opponent()
        };
        // White's chances fall when White errs, rise when Black does.
        let accuracy = match mover {
            Side::White => move_accuracy(pair[0], pair[1]),
            Side::Black => move_accuracy(pair[1], pair[0]),
        };
        by_side[side_index(mover)].push((accuracy, weight));
    }
    by_side.map(|moves| side_accuracy(&moves))
}

/// How sharp the game was around each move: the deviation of the winning chances over a window
/// of positions. The first moves share the first window, as Lichess pads them.
fn volatility_weights(wins: &[f64]) -> Vec<f64> {
    let size = window_size(wins.len());
    let padding = size.min(wins.len()).saturating_sub(2);
    let first = &wins[..size.min(wins.len())];
    std::iter::repeat_n(first, padding)
        .chain(wins.windows(size))
        .map(|window| population_deviation(window).clamp(WEIGHT.0, WEIGHT.1))
        .collect()
}

/// A tenth of the moves, between two and eight positions.
fn window_size(positions: usize) -> usize {
    (positions.saturating_sub(1) / 10).clamp(WINDOW.0, WINDOW.1)
}

/// The mean of the volatility-weighted mean and the harmonic mean of a side's moves.
fn side_accuracy(moves: &[(f64, f64)]) -> Option<f64> {
    let weighted = weighted_mean(moves)?;
    let accuracies: Vec<f64> = moves.iter().map(|(accuracy, _)| *accuracy).collect();
    let harmonic = harmonic_mean(&accuracies)?;
    Some((weighted + harmonic) / 2.0)
}

fn weighted_mean(values: &[(f64, f64)]) -> Option<f64> {
    let (sum, weights) = values
        .iter()
        .fold((0.0, 0.0), |(sum, weights), (value, weight)| {
            (sum + value * weight, weights + weight)
        });
    (weights != 0.0).then(|| sum / weights)
}

/// Each value counts as at least 1, so a single 0 does not swallow the rest.
fn harmonic_mean(values: &[f64]) -> Option<f64> {
    if values.is_empty() {
        return None;
    }
    let inverses: f64 = values.iter().map(|value| 1.0 / value.max(1.0)).sum();
    Some(count(values) / inverses)
}

/// The deviation over the whole window, dividing by its size.
fn population_deviation(values: &[f64]) -> f64 {
    let mean = values.iter().sum::<f64>() / count(values);
    let variance = values
        .iter()
        .map(|value| (value - mean).powi(2))
        .sum::<f64>()
        / count(values);
    variance.sqrt()
}

fn count(values: &[f64]) -> f64 {
    f64::from(u32::try_from(values.len()).unwrap_or(u32::MAX))
}

fn side_index(side: Side) -> usize {
    match side {
        Side::White => 0,
        Side::Black => 1,
    }
}

#[cfg(test)]
mod tests {
    use chess_domain::{Side, engine::Eval};

    use super::{game_accuracy, move_accuracy, win_percent};

    /// Lichess's starting value for a game: White slightly better.
    const START: i32 = 15;

    /// A game's evaluations as Lichess exports them, in pawns, White's view; `#-2` a mate.
    fn evals(pawns: &str) -> Vec<Eval> {
        std::iter::once(Eval::Centipawns(START))
            .chain(
                pawns
                    .split_whitespace()
                    .map(|value| match value.strip_prefix('#') {
                        Some(mate) => Eval::Mate {
                            winner: if mate.starts_with('-') {
                                Side::Black
                            } else {
                                Side::White
                            },
                            moves: mate.trim_start_matches('-').parse().expect("a mate"),
                        },
                        None => {
                            let pawns: f64 = value.parse().expect("pawns");
                            #[expect(clippy::cast_possible_truncation, reason = "a few thousand")]
                            let centipawns = (pawns * 100.0).round() as i32;
                            Eval::Centipawns(centipawns)
                        }
                    }),
            )
            .collect()
    }

    fn assert_close(actual: Option<f64>, expected: f64) {
        let actual = actual.expect("an accuracy");
        assert!(
            (actual - expected).abs() < 0.001,
            "{actual} is not {expected}"
        );
    }

    #[test]
    fn a_long_lichess_game_has_the_accuracy_lichess_gives_it() {
        // lichess.org/iSjpt213: Lichess shows 93 for White and 87 for Black.
        let game = evals(
            "0.2 0.17 0.29 0.32 0.21 0.76 0.56 0.75 0.13 0.43 0.13 0.7 0.5 0.42 0.33 0.81 0.48 \
             0.52 0.1 2.53 1.78 2.73 3.05 3.18 1.64 4.46 4.46 4.78 4.53 5.26 5.7 5.72 5.79 5.79 \
             4.17 3.96 3.88 3.92 4.02 5.16 4.31 4.5 3.42 4.62 4.37 4.85 3.98 4.0 4.09 3.97 4.0 \
             6.02 5.9 5.9 6.03 6.08 5.79 5.91 5.92 5.94 5.72 5.64 5.08 5.35 5.25 5.67 5.45 5.51 \
             5.55 5.7 5.16 5.79 4.97 5.29 5.06 5.57 5.53 5.54 5.51 5.62 5.57 5.57 5.14 6.07 5.71 \
             5.71 5.41 5.74 5.83 5.86 5.59 5.7 4.51 5.74 5.61 5.57 5.6 5.6 5.83 5.71 5.61 5.56 \
             5.48 5.57 5.44 9.23 34.76",
        );
        let [white, black] = game_accuracy(&game, Side::White);
        assert_close(white, 92.674);
        assert_close(black, 87.431);
    }

    #[test]
    fn a_game_lost_to_mate_has_the_accuracy_lichess_gives_it() {
        // lichess.org/FqwXiMEE, mated after this: Lichess shows 60 for White and 77 for Black.
        let game = evals(
            "0.2 0.17 0.29 1.23 0.38 0.62 0.28 6.03 0.0 0.0 0.0 0.08 -0.09 0.07 -0.34 -0.24 \
             -0.77 0.51 -0.37 -0.4 -0.92 -0.82 -0.96 -0.66 -0.68 -0.58 -0.76 0.18 -0.54 0.09 \
             -2.61 -2.55 -4.67 -4.67 #-1 #-2 #-2 #-1 #-1",
        );
        let [white, black] = game_accuracy(&game, Side::White);
        assert_close(white, 60.175);
        assert_close(black, 76.674);
    }

    #[test]
    fn a_move_that_keeps_its_chances_is_perfect_and_one_that_throws_them_away_is_not() {
        assert!((move_accuracy(60.0, 60.0) - 100.0).abs() < f64::EPSILON);
        assert!((move_accuracy(40.0, 70.0) - 100.0).abs() < f64::EPSILON);
        assert!(move_accuracy(80.0, 20.0) < 10.0);
        assert!(
            move_accuracy(97.5, 2.5).abs() < f64::EPSILON,
            "never below zero"
        );
    }

    #[test]
    fn a_mate_counts_as_ten_pawns() {
        let mate = Eval::Mate {
            winner: Side::White,
            moves: 3,
        };
        assert!((win_percent(mate) - win_percent(Eval::Centipawns(1000))).abs() < 1e-12);
        assert!((win_percent(Eval::Centipawns(5000)) - win_percent(mate)).abs() < 1e-12);
        assert!((win_percent(Eval::Centipawns(0)) - 50.0).abs() < 1e-12);
    }

    #[test]
    fn a_side_that_never_moved_has_no_accuracy() {
        let [white, black] = game_accuracy(
            &[Eval::Centipawns(START), Eval::Centipawns(20)],
            Side::White,
        );
        assert!(white.is_some());
        assert_eq!(black, None);
    }
}
