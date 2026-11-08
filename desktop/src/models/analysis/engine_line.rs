use chess_domain::engine::Eval;

/// One of the engine's lines, ready to show.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LineView {
    pub eval: Eval,
    pub moves: String,
}
