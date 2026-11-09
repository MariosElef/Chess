use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::{challenge::ChallengeModel, game::GameSnapshot};
use crate::models::{
    chat::message::ChatMessageModel,
    puzzles::battle::{BattleChallengeModel, BattleSnapshot},
};

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
    /// Pair me with the next player who wants a Puzzle Battle, rated or not.
    SeekBattle {
        rated: bool,
    },
    CancelBattleSeek,
    /// The player's answer to their current puzzle in the battle: solved, or a mistake.
    BattleAnswer {
        battle_id: Uuid,
        solved: bool,
    },
    /// Invite a friend who is online to a Puzzle Battle.
    ChallengeBattle {
        user_id: Uuid,
        rated: bool,
    },
    /// Accept a battle invitation.
    AcceptBattle {
        challenge_id: Uuid,
    },
    /// Decline a battle invitation, or withdraw one sent.
    DeclineBattle {
        challenge_id: Uuid,
    },
    /// Give up a battle being played: the other player wins.
    ResignBattle {
        battle_id: Uuid,
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
    /// The player's own account changed on another device, as its picture: fetch it again.
    ProfileChanged,
    /// Waiting for a Puzzle Battle opponent.
    BattleSeeking,
    BattleSeekCancelled,
    /// A battle the player is in, as it stands now.
    Battle {
        battle: BattleSnapshot,
    },
    /// The battle invitations waiting, each way, whenever they change.
    BattleChallenges {
        sent: Vec<BattleChallengeModel>,
        received: Vec<BattleChallengeModel>,
    },
}
