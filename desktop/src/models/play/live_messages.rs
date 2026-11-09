use serde::{Deserialize, Serialize};

use super::{challenge::Challenge, live_game::GameSnapshot};
use crate::models::{
    chat::ChatMessage,
    puzzles::{BattleChallenge, BattleSnapshot},
};

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
    /// Invite a friend who is online to a game.
    Challenge {
        user_id: String,
        initial_s: u32,
        increment_s: u32,
        rated: bool,
    },
    /// Withdraw a challenge sent.
    CancelChallenge {
        challenge_id: String,
    },
    AcceptChallenge {
        challenge_id: String,
    },
    /// Play the game just finished again, colours swapped; if they asked already, it starts.
    OfferRematch {
        game_id: String,
    },
    /// Accept the opponent's rematch of that game; refused as that game's if it is gone.
    AcceptRematch {
        game_id: String,
    },
    /// Withdraw the rematch of that game the player asked for, if it is still waiting.
    CancelRematch {
        game_id: String,
    },
    DeclineChallenge {
        challenge_id: String,
    },
    /// Pair me with the next player who wants a Puzzle Battle.
    SeekBattle {
        rated: bool,
    },
    CancelBattleSeek,
    /// The answer to the current puzzle of the battle: solved, or a mistake.
    BattleAnswer {
        battle_id: String,
        solved: bool,
    },
    /// Invite a friend who is online to a battle.
    ChallengeBattle {
        user_id: String,
        rated: bool,
    },
    AcceptBattle {
        challenge_id: String,
    },
    /// Decline an invitation, or withdraw one sent.
    DeclineBattle {
        challenge_id: String,
    },
    /// Give up the battle: the other player wins.
    ResignBattle {
        battle_id: String,
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
    /// A friend came or went, or a friendship or request changed: fetch the friends again.
    FriendsChanged,
    /// The challenges waiting, each way, whenever they change.
    Challenges {
        sent: Vec<Challenge>,
        received: Vec<Challenge>,
    },
    /// A message in one of the player's conversations.
    ChatMessage {
        message: ChatMessage,
    },
    /// A conversation was started, or someone joined or left one: fetch the list again.
    ConversationsChanged,
    /// The player's own account changed on another device, as its picture: fetch it again.
    ProfileChanged,
    /// A rematch of the game asked for or accepted was refused, in a sentence.
    RematchRefused {
        game_id: String,
        message: String,
    },
    /// Waiting for a Puzzle Battle opponent.
    BattleSeeking,
    BattleSeekCancelled,
    /// A battle the player is in, as it stands now.
    Battle {
        battle: Box<BattleSnapshot>,
    },
    /// The battle invitations waiting, each way.
    BattleChallenges {
        sent: Vec<BattleChallenge>,
        received: Vec<BattleChallenge>,
    },
    /// Accepting a friend's challenge or a battle invitation was refused, in a sentence.
    InviteRefused {
        challenge_id: String,
        message: String,
    },
}

/// What happens on the live socket: the connection coming and going, and server messages.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LiveEvent {
    Connected,
    /// The connection dropped; it is tried again on its own.
    Disconnected,
    /// The server no longer takes the session, as when another app signed it out: no retrying.
    SignedOut,
    Message(Box<ServerMessage>),
}
