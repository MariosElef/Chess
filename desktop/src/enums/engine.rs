//! What arrives from the Stockfish process.

use chess_domain::engine::UciMessage;

/// What arrives from the engine process.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EngineEvent {
    Message(UciMessage),
    /// Its output ended: it exited or crashed.
    Stopped,
}
