#![expect(
    clippy::float_arithmetic,
    reason = "win chances are a logistic curve over the centipawn score"
)]

use super::UciScore;
use crate::Side;

/// Lichess's fit from centipawns to winning chances.
const WIN_CURVE: f64 = 0.003_682_08;
/// The bar never fills completely without a forced mate, so both colours stay visible.
const MIN_SHARE: f64 = 0.05;
const MAX_SHARE: f64 = 0.95;

/// What a position is worth from White's point of view, however the engine reported it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Eval {
    /// Hundredths of a pawn; positive is good for White.
    Centipawns(i32),
    /// A forced mate: who delivers it, and in how many of their moves (0 when already mated).
    Mate { winner: Side, moves: u32 },
}

impl Eval {
    /// Turns the engine's side-to-move score into White's point of view.
    pub fn from_uci(score: UciScore, side_to_move: Side) -> Self {
        match score {
            UciScore::Cp(centipawns) => Self::Centipawns(for_white(centipawns, side_to_move)),
            UciScore::Mate(moves) if moves > 0 => Self::Mate {
                winner: side_to_move,
                moves: moves.unsigned_abs(),
            },
            UciScore::Mate(moves) => Self::Mate {
                winner: side_to_move.opponent(),
                moves: moves.unsigned_abs(),
            },
        }
    }

    /// `+1.3`, `-0.5`, `0.0`, or `M5` / `-M3` for a mate by White / Black.
    pub fn label(self) -> String {
        match self {
            Self::Centipawns(centipawns) => pawns_label(centipawns),
            Self::Mate {
                winner: Side::White,
                moves,
            } => format!("M{moves}"),
            Self::Mate {
                winner: Side::Black,
                moves,
            } => format!("-M{moves}"),
        }
    }

    /// How much of the eval bar is White's, from 0 to 1.
    pub fn white_share(self) -> f64 {
        match self {
            Self::Centipawns(centipawns) => {
                ((1.0 + win_chances(centipawns)) / 2.0).clamp(MIN_SHARE, MAX_SHARE)
            }
            Self::Mate {
                winner: Side::White,
                ..
            } => 1.0,
            Self::Mate {
                winner: Side::Black,
                ..
            } => 0.0,
        }
    }
}

fn for_white(centipawns: i32, side_to_move: Side) -> i32 {
    match side_to_move {
        Side::White => centipawns,
        Side::Black => -centipawns,
    }
}

/// From -1 (Black is winning) to +1 (White is winning).
fn win_chances(centipawns: i32) -> f64 {
    2.0 / (1.0 + (-WIN_CURVE * f64::from(centipawns)).exp()) - 1.0
}

/// One decimal of a pawn, with a sign when not level: 130 → `+1.3`.
fn pawns_label(centipawns: i32) -> String {
    let tenths = (f64::from(centipawns) / 10.0).round();
    let pawns = tenths / 10.0;
    if tenths == 0.0 {
        "0.0".to_owned()
    } else {
        format!("{pawns:+.1}")
    }
}

#[cfg(test)]
mod tests {
    use super::Eval;
    use crate::{Side, engine::UciScore};

    #[test]
    fn black_to_move_scores_are_turned_around() {
        assert_eq!(
            Eval::from_uci(UciScore::Cp(50), Side::White),
            Eval::Centipawns(50)
        );
        assert_eq!(
            Eval::from_uci(UciScore::Cp(50), Side::Black),
            Eval::Centipawns(-50)
        );
    }

    #[test]
    fn mates_name_the_side_that_delivers_them() {
        assert_eq!(
            Eval::from_uci(UciScore::Mate(3), Side::Black),
            Eval::Mate {
                winner: Side::Black,
                moves: 3
            }
        );
        assert_eq!(
            Eval::from_uci(UciScore::Mate(-2), Side::Black),
            Eval::Mate {
                winner: Side::White,
                moves: 2
            }
        );
        // Mate 0: the side to move is already mated.
        assert_eq!(
            Eval::from_uci(UciScore::Mate(0), Side::White),
            Eval::Mate {
                winner: Side::Black,
                moves: 0
            }
        );
    }

    #[test]
    fn labels_read_like_lichess() {
        assert_eq!(Eval::Centipawns(130).label(), "+1.3");
        assert_eq!(Eval::Centipawns(-48).label(), "-0.5");
        assert_eq!(Eval::Centipawns(3).label(), "0.0");
        assert_eq!(
            Eval::Mate {
                winner: Side::White,
                moves: 5
            }
            .label(),
            "M5"
        );
        assert_eq!(
            Eval::Mate {
                winner: Side::Black,
                moves: 3
            }
            .label(),
            "-M3"
        );
    }

    #[test]
    fn the_bar_follows_winning_chances() {
        assert!((Eval::Centipawns(0).white_share() - 0.5).abs() < 1e-9);
        assert!(Eval::Centipawns(200).white_share() > 0.6);
        assert!((Eval::Centipawns(5000).white_share() - 0.95).abs() < 1e-9);
        assert!((Eval::Centipawns(-5000).white_share() - 0.05).abs() < 1e-9);
        assert!(
            (Eval::Mate {
                winner: Side::White,
                moves: 1
            }
            .white_share()
                - 1.0)
                .abs()
                < 1e-9
        );
        assert!(
            Eval::Mate {
                winner: Side::Black,
                moves: 1
            }
            .white_share()
            .abs()
                < 1e-9
        );
    }
}
