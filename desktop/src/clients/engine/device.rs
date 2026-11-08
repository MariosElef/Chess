use std::{num::NonZero, thread};

/// How many cores this computer has, for Stockfish's thread choices and defaults.
pub fn device_cores() -> usize {
    thread::available_parallelism().map_or(1, NonZero::get)
}
