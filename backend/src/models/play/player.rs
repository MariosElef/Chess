use serde::Serialize;
use uuid::Uuid;

/// Someone connected to play: who they are, as their opponent sees them.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Player {
    pub id: Uuid,
    pub username: String,
    /// A shared guest account: it plays unrated only.
    pub guest: bool,
    /// Their picture's hash, as their friends and chats see it.
    pub avatar: Option<String>,
}

/// A player at the board, with their rating in the game's category; none for a guest.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PlayerModel {
    pub id: Uuid,
    pub username: String,
    pub rating: Option<i32>,
}
