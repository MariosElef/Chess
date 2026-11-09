use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::enums::social::Relation;

/// Another player, by name.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct UserRef {
    pub id: Uuid,
    pub username: String,
    /// Their picture's hash, which names it at `/api/avatars/{id}/{hash}`; none without one.
    pub avatar: Option<String>,
}

/// A player found by search, and where the searcher stands with them.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct FoundUserModel {
    pub id: Uuid,
    pub username: String,
    pub avatar: Option<String>,
    pub relation: Relation,
}

/// A friend: whether they have the app open now, when they last had, and their ratings.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct FriendModel {
    pub id: Uuid,
    pub username: String,
    pub avatar: Option<String>,
    pub online: bool,
    /// Seconds since they last had the app open or used a session; `None` if neither is known.
    pub away_s: Option<i64>,
    /// Their rating in each category they have one, by the category's id.
    pub ratings: BTreeMap<String, i32>,
}

/// The Social tab: friends, and the requests each way.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct GetFriendsModel {
    pub friends: Vec<FriendModel>,
    /// The signed-in player's own ratings, for the friends' leaderboard.
    pub my_ratings: BTreeMap<String, i32>,
    /// Asked by others, waiting for the signed-in player.
    pub incoming: Vec<UserRef>,
    /// Asked by the signed-in player, waiting for the others.
    pub outgoing: Vec<UserRef>,
    /// Those the signed-in player has blocked.
    pub blocked: Vec<UserRef>,
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

/// A player found by search, and the friendship with them, if any.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FriendshipRow {
    pub user_id: Uuid,
    pub username: String,
    pub avatar: Option<String>,
    pub status: Option<String>,
    pub requester_id: Option<Uuid>,
    /// The searcher has blocked them.
    pub blocked: bool,
}

/// A friend or someone with a request each way, the friendship, and when they were last online.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ListedFriendRow {
    pub user_id: Uuid,
    pub username: String,
    pub avatar: Option<String>,
    /// Seconds since they last had the app open or used a session; `None` if neither is known.
    pub away_s: Option<i64>,
    pub status: Option<String>,
    pub requester_id: Option<Uuid>,
}
