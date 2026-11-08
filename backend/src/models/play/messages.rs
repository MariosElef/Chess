use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::game::GameSnapshot;

/// What the app sends over the live socket.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ClientMessage {
    /// Pair me with the next player asking for the same time control.
    Seek {
        initial_s: u32,
        increment_s: u32,
        rated: bool,
    },
    CancelSeek,
    Move {
        game_id: Uuid,
        uci: String,
    },
    Resign {
        game_id: Uuid,
    },
    /// Call the game off; only before both sides have moved.
    Abort {
        game_id: Uuid,
    },
    OfferDraw {
        game_id: Uuid,
    },
    AcceptDraw {
        game_id: Uuid,
    },
    DeclineDraw {
        game_id: Uuid,
    },
}

/// What the server sends over the live socket.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ServerMessage {
    /// Waiting for an opponent.
    Seeking {
        initial_s: u32,
        increment_s: u32,
        rated: bool,
    },
    SeekCancelled,
    /// A game the player is in, as it stands now.
    Game {
        game: GameSnapshot,
    },
    /// A message refused, in a sentence.
    Error {
        message: String,
    },
}
