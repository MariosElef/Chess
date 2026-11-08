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
    notation::{FenError, PgnError},
};
pub use models::{
    board::{Board, CastlingRights, Piece, Square},
    game::{Game, MoveCounters, Position, piece_value},
    moves::Move,
    rating::{RatedResult, Rating},
    time::{Clock, TimeControl},
};

/// Talking to a UCI engine such as Stockfish: commands sent, lines read back, what scores mean.
pub mod engine {
    pub use crate::{
        enums::engine::{Command, Eval, UciMessage, UciScore},
        models::engine::SearchInfo,
    };
}
