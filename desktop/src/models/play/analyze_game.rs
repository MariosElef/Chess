use chess_domain::{Game, Outcome};

/// Play asks for its game to be opened on the analysis board, at the position it was showing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AnalyzeGame {
    pub game: Game,
    pub viewing: usize,
    /// Game Review was asked for: the analysis board reviews it straight away.
    pub review: bool,
    /// How it ended, when the moves alone do not say: a resignation, a timeout, a draw agreed.
    pub outcome: Option<Outcome>,
}
