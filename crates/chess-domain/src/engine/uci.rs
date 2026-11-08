use std::fmt;

use crate::Move;

/// What the engine says a position is worth, from the side to move's point of view.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum UciScore {
    /// Hundredths of a pawn.
    Cp(i32),
    /// Mate in this many moves; negative when the side to move is the one being mated.
    Mate(i32),
}

/// One `info` line of a search: how deep it went and the line it found.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchInfo {
    pub depth: u32,
    /// Which of the best lines this is, 1 for the best.
    pub multipv: usize,
    pub score: UciScore,
    pub nodes: u64,
    pub nps: u64,
    pub pv: Vec<Move>,
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

/// An `info` line worth showing: it has a score that is exact and a line to play. Bound scores
/// (`lowerbound`/`upperbound`) are interim guesses, and `info string` lines are messages.
fn parse_info<'a>(mut words: impl Iterator<Item = &'a str>) -> Option<SearchInfo> {
    let mut fields = InfoFields::default();
    while let Some(word) = words.next() {
        match word {
            "string" | "lowerbound" | "upperbound" => return None,
            "depth" => fields.depth = number(words.next()),
            "multipv" => fields.multipv = number(words.next()),
            "nodes" => fields.nodes = number(words.next()),
            "nps" => fields.nps = number(words.next()),
            "score" => fields.score = score(words.next(), words.next()),
            "pv" => fields.pv = words.by_ref().map_while(|mv| mv.parse().ok()).collect(),
            _ => {}
        }
    }
    fields.complete()
}

#[derive(Default)]
struct InfoFields {
    depth: Option<u32>,
    multipv: Option<usize>,
    score: Option<UciScore>,
    nodes: Option<u64>,
    nps: Option<u64>,
    pv: Vec<Move>,
}

impl InfoFields {
    fn complete(self) -> Option<SearchInfo> {
        if self.pv.is_empty() {
            return None;
        }
        Some(SearchInfo {
            depth: self.depth?,
            multipv: self.multipv.unwrap_or(1),
            score: self.score?,
            nodes: self.nodes.unwrap_or(0),
            nps: self.nps.unwrap_or(0),
            pv: self.pv,
        })
    }
}

fn number<T: std::str::FromStr>(word: Option<&str>) -> Option<T> {
    word?.parse().ok()
}

fn score(kind: Option<&str>, value: Option<&str>) -> Option<UciScore> {
    match kind? {
        "cp" => number(value).map(UciScore::Cp),
        "mate" => number(value).map(UciScore::Mate),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::{Command, SearchInfo, UciMessage, UciScore};
    use crate::Move;

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
