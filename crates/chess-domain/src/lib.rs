//! The rules of chess, shared by the desktop app and the server: plain Rust, no UI, no I/O.
mod enums;
mod errors;
mod models;

pub use enums::{
    board::{PieceKind, Side},
    game::{GameStatus, Outcome},
    rating::Score,
    time::Category,
};
pub use errors::{
    board::ParseSquareError,
    game::GameError,
    moves::{IllegalMove, ParseMoveError},
    notation::FenError,
};
pub use models::{
    board::{Board, CastlingRights, Piece, Square},
    game::{Game, MoveCounters, Position, piece_value},
    moves::Move,
    rating::{RatedResult, Rating},
    time::{Clock, TimeControl},
};

/// Talking to a UCI chess engine such as Stockfish: the commands sent, the lines read back, and
/// what an evaluation means. Pure text and numbers; whoever runs the engine process uses these.
pub mod engine {
    pub use crate::{
        enums::engine::{Command, Eval, UciMessage, UciScore},
        models::engine::SearchInfo,
    };
}
