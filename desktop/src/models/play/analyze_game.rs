use chess_domain::Game;

/// Play asks for its game to be opened on the analysis board, at the position it was showing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AnalyzeGame {
    pub game: Game,
    pub viewing: usize,
}
