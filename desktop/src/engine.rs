//! Running Stockfish next to the app: finding it, starting it, and keeping one search going.
mod locate;
mod search;
mod settings;
mod stockfish;

pub use locate::locate_stockfish;
pub use search::SearchQueue;
pub use settings::{
    DEPTH_CHOICES, EngineSettings, HASH_CHOICES, depth_label, device_cores, thread_choices,
};
pub use stockfish::{Engine, EngineEvent};
