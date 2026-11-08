use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::{challenge::ChallengeModel, game::GameSnapshot};
use crate::models::chat::message::ChatMessageModel;

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
    /// Invite a friend who is online to a game.
    Challenge {
        user_id: Uuid,
        initial_s: u32,
        increment_s: u32,
        rated: bool,
    },
    /// Withdraw a challenge sent.
    CancelChallenge {
        challenge_id: Uuid,
    },
    AcceptChallenge {
        challenge_id: Uuid,
    },
    DeclineChallenge {
        challenge_id: Uuid,
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
    /// A friend came or went, or a friendship or request changed: fetch the friends again.
    FriendsChanged,
    /// The challenges waiting, each way, whenever they change.
    Challenges {
        sent: Vec<ChallengeModel>,
        received: Vec<ChallengeModel>,
    },
    /// A message in one of the player's conversations.
    ChatMessage {
        message: ChatMessageModel,
    },
    /// A conversation was started, or someone joined or left one: fetch the list again.
    ConversationsChanged,
}
