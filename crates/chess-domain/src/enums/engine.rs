//! What an engine says: its scores, the lines it sends, the commands it takes, and what an
//! evaluation means for White.
#![expect(
    clippy::float_arithmetic,
    reason = "win chances are a logistic curve over the centipawn score"
)]

use std::fmt;

use crate::{
    Move, Side,
    models::engine::{SearchInfo, parse_info},
};

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

    /// The share of the points `side` can expect from here, from 0 (lost) through 0.5 (even) to
    /// 1 (won): Lichess's winning-chances curve, with a forced mate as a sure result.
    pub fn expected_points(self, side: Side) -> f64 {
        let for_white = match self {
            Self::Centipawns(centipawns) => (1.0 + win_chances(centipawns)) / 2.0,
            Self::Mate {
                winner: Side::White,
                ..
            } => 1.0,
            Self::Mate {
                winner: Side::Black,
                ..
            } => 0.0,
        };
        match side {
            Side::White => for_white,
            Side::Black => 1.0 - for_white,
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

/// What the engine says a position is worth, from the side to move's point of view.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum UciScore {
    /// Hundredths of a pawn.
    Cp(i32),
    /// Mate in this many moves; negative when the side to move is the one being mated.
    Mate(i32),
}

/// The engine lines a GUI acts on; everything else is `Other`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UciMessage {
    UciOk,
    ReadyOk,
    Info(SearchInfo),
    /// The search ended; `None` when the position has no legal move.
    BestMove(Option<Move>),
    Other,
}

impl UciMessage {
    /// Reads one line from the engine.
    pub fn parse(line: &str) -> Self {
        let mut words = line.split_whitespace();
        match words.next() {
            Some("uciok") => Self::UciOk,
            Some("readyok") => Self::ReadyOk,
            Some("bestmove") => Self::BestMove(words.next().and_then(|mv| mv.parse().ok())),
            Some("info") => parse_info(words).map_or(Self::Other, Self::Info),
            _ => Self::Other,
        }
    }
}

/// What the GUI sends; `Display` writes the exact line the engine expects.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Command {
    Uci,
    SetOption(&'static str, String),
    IsReady,
    UciNewGame,
    /// A position given as FEN.
    Position(String),
    GoInfinite,
    /// Search until this depth, then report the best move.
    GoDepth(u32),
    Stop,
    Quit,
}

impl fmt::Display for Command {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Uci => f.write_str("uci"),
            Self::SetOption(name, value) => write!(f, "setoption name {name} value {value}"),
            Self::IsReady => f.write_str("isready"),
            Self::UciNewGame => f.write_str("ucinewgame"),
            Self::Position(fen) => write!(f, "position fen {fen}"),
            Self::GoInfinite => f.write_str("go infinite"),
            Self::GoDepth(depth) => write!(f, "go depth {depth}"),
            Self::Stop => f.write_str("stop"),
            Self::Quit => f.write_str("quit"),
        }
    }
}

impl Command {
    /// Whether this starts a search.
    pub fn is_go(&self) -> bool {
        matches!(self, Self::GoInfinite | Self::GoDepth(_))
    }
}

#[cfg(test)]
mod tests {
    use super::Eval;
    use crate::{Side, engine::UciScore};

    #[test]
    fn expected_points_are_even_at_zero_and_sure_with_a_mate() {
        assert!((Eval::Centipawns(0).expected_points(Side::White) - 0.5).abs() < 1e-9);
        let up_a_pawn = Eval::Centipawns(100);
        assert!(up_a_pawn.expected_points(Side::White) > 0.55);
        let both = up_a_pawn.expected_points(Side::White) + up_a_pawn.expected_points(Side::Black);
        assert!((both - 1.0).abs() < 1e-9, "the two sides share one point");
        let mate = Eval::Mate {
            winner: Side::Black,
            moves: 3,
        };
        assert!((mate.expected_points(Side::Black) - 1.0).abs() < 1e-9);
        assert!(mate.expected_points(Side::White).abs() < 1e-9);
    }

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

#[cfg(test)]
mod uci_tests {
    use super::{Command, UciMessage, UciScore};
    use crate::{Move, engine::SearchInfo};

    fn moves(text: &str) -> Vec<Move> {
        text.split_whitespace()
            .map(|mv| mv.parse().expect("a UCI move"))
            .collect()
    }

    // Copied from Stockfish 19's output for the starting position.
    #[test]
    fn a_real_info_line_is_read_in_full() {
        let line = "info depth 12 seldepth 16 multipv 2 score cp 17 nodes 53793 nps 625500 \
                    hashfull 11 tbhits 0 time 86 pv g1f3 g8f6 c2c4 e7e6";
        assert_eq!(
            UciMessage::parse(line),
            UciMessage::Info(SearchInfo {
                depth: 12,
                multipv: 2,
                score: UciScore::Cp(17),
                nodes: 53793,
                nps: 625_500,
                pv: moves("g1f3 g8f6 c2c4 e7e6"),
            })
        );
    }

    #[test]
    fn a_line_without_multipv_is_the_best_line() {
        let line = "info depth 20 score mate -3 nodes 10 nps 5 pv e1e2";
        let UciMessage::Info(info) = UciMessage::parse(line) else {
            panic!("an info line");
        };
        assert_eq!(info.multipv, 1);
        assert_eq!(info.score, UciScore::Mate(-3));
    }

    #[test]
    fn guesses_messages_and_lines_without_a_pv_are_skipped() {
        for line in [
            "info depth 10 score cp 20 lowerbound nodes 1 nps 1 pv e2e4",
            "info string Using 1 thread",
            "info depth 1 currmove e2e4 currmovenumber 1",
            "info depth 5 score cp 10 nodes 1 nps 1",
        ] {
            assert_eq!(UciMessage::parse(line), UciMessage::Other, "{line}");
        }
    }

    #[test]
    fn handshake_and_bestmove_lines() {
        assert_eq!(UciMessage::parse("uciok"), UciMessage::UciOk);
        assert_eq!(UciMessage::parse("readyok"), UciMessage::ReadyOk);
        assert_eq!(
            UciMessage::parse("bestmove e2e4 ponder e7e5"),
            UciMessage::BestMove(Some("e2e4".parse().expect("valid")))
        );
        assert_eq!(
            UciMessage::parse("bestmove (none)"),
            UciMessage::BestMove(None)
        );
        assert_eq!(UciMessage::parse("id name Stockfish 19"), UciMessage::Other);
    }

    #[test]
    fn commands_are_written_as_uci_expects() {
        let fen = "8/8/8/8/8/8/8/k6K w - - 0 1".to_owned();
        let lines: Vec<String> = [
            Command::Uci,
            Command::SetOption("MultiPV", "3".to_owned()),
            Command::IsReady,
            Command::UciNewGame,
            Command::Position(fen),
            Command::GoInfinite,
            Command::GoDepth(24),
            Command::Stop,
            Command::Quit,
        ]
        .iter()
        .map(ToString::to_string)
        .collect();
        assert_eq!(
            lines,
            [
                "uci",
                "setoption name MultiPV value 3",
                "isready",
                "ucinewgame",
                "position fen 8/8/8/8/8/8/8/k6K w - - 0 1",
                "go infinite",
                "go depth 24",
                "stop",
                "quit",
            ]
        );
    }
}
