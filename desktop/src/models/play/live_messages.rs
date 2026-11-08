use serde::{Deserialize, Serialize};

use super::live_game::GameSnapshot;

/// What the app sends over the live socket.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ClientMessage {
    Seek {
        initial_s: u32,
        increment_s: u32,
        rated: bool,
    },
    CancelSeek,
    Move {
        game_id: String,
        uci: String,
    },
    Resign {
        game_id: String,
    },
    /// Call the game off; only before both sides have moved.
    Abort {
        game_id: String,
    },
    OfferDraw {
        game_id: String,
    },
    AcceptDraw {
        game_id: String,
    },
    DeclineDraw {
        game_id: String,
    },
}

/// What the server sends over the live socket.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ServerMessage {
    Seeking {
        initial_s: u32,
        increment_s: u32,
        rated: bool,
    },
    SeekCancelled,
    Game {
        game: GameSnapshot,
    },
    Error {
        message: String,
    },
}

/// What happens on the live socket, for the page: the connection coming and going, and the
/// server's messages.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LiveEvent {
    Connected,
    /// The connection dropped; it is tried again on its own.
    Disconnected,
    Message(Box<ServerMessage>),
}
