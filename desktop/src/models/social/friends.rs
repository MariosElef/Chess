use serde::Deserialize;

use crate::enums::social::Relation;

/// Another player, by name.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct UserRef {
    pub id: String,
    pub username: String,
}

/// A player found by search, and where the searcher stands with them.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct FoundUser {
    pub id: String,
    pub username: String,
    pub relation: Relation,
}

/// A friend, and whether they have the app open now.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Friend {
    pub id: String,
    pub username: String,
    pub online: bool,
}

/// Friends, online first, and the requests each way.
#[derive(Debug, Clone, PartialEq, Eq, Default, Deserialize)]
pub struct Friends {
    pub friends: Vec<Friend>,
    /// Asked by others, waiting for the signed-in player.
    pub incoming: Vec<UserRef>,
    /// Asked by the signed-in player, waiting for the others.
    pub outgoing: Vec<UserRef>,
}
