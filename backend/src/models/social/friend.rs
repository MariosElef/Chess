use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::enums::social::Relation;

/// Another player, by name.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct UserRef {
    pub id: Uuid,
    pub username: String,
}

/// A player found by search, and where the searcher stands with them.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct FoundUserModel {
    pub id: Uuid,
    pub username: String,
    pub relation: Relation,
}

/// A friend, and whether they have the app open now.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct FriendModel {
    pub id: Uuid,
    pub username: String,
    pub online: bool,
}

/// The Social tab: friends, and the requests each way.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct GetFriendsModel {
    pub friends: Vec<FriendModel>,
    /// Asked by others, waiting for the signed-in player.
    pub incoming: Vec<UserRef>,
    /// Asked by the signed-in player, waiting for the others.
    pub outgoing: Vec<UserRef>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct CreateFriendRequestModel {
    pub user_id: Uuid,
}

/// Where the two now stand, after a request.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct GetRelationModel {
    pub relation: Relation,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct UserSearchQuery {
    pub q: String,
}

/// How one pair stands: the stored status and who asked.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PairRow {
    pub status: String,
    pub requester_id: Uuid,
}

/// A row about another player and the friendship with them, if any.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FriendshipRow {
    pub user_id: Uuid,
    pub username: String,
    pub status: Option<String>,
    pub requester_id: Option<Uuid>,
}
