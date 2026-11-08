//! Talking to a UCI chess engine such as Stockfish: the commands sent, the lines read back, and
//! what an evaluation means. Pure text and numbers; whoever runs the engine process uses these.
mod eval;
mod uci;

pub use eval::Eval;
pub use uci::{Command, SearchInfo, UciMessage, UciScore};
