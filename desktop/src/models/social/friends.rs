use std::collections::BTreeMap;

use serde::Deserialize;

use crate::enums::social::Relation;

/// Another player, by name.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct UserRef {
    pub id: String,
    pub username: String,
    /// Their picture's hash; none while they show their initial.
    #[serde(default)]
    pub avatar: Option<String>,
}

/// A player found by search, and where the searcher stands with them.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct FoundUser {
    pub id: String,
    pub username: String,
    #[serde(default)]
    pub avatar: Option<String>,
    pub relation: Relation,
}

/// A friend: whether they have the app open now, how long since they last had, and ratings.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Friend {
    pub id: String,
    pub username: String,
    #[serde(default)]
    pub avatar: Option<String>,
    pub online: bool,
    /// Seconds since they last had the app open or used a session; `None` if neither is known.
    #[serde(default)]
    pub away_s: Option<i64>,
    /// Their rating in each category they have one, by the category's id.
    #[serde(default)]
    pub ratings: BTreeMap<String, i32>,
}

/// Friends and the requests each way, with the player's own ratings for the leaderboard.
#[derive(Debug, Clone, PartialEq, Eq, Default, Deserialize)]
pub struct Friends {
    pub friends: Vec<Friend>,
    /// The signed-in player's own ratings, for the friends' leaderboard.
    #[serde(default)]
    pub my_ratings: BTreeMap<String, i32>,
    /// Asked by others, waiting for the signed-in player.
    pub incoming: Vec<UserRef>,
    /// Asked by the signed-in player, waiting for the others.
    pub outgoing: Vec<UserRef>,
    /// Those the signed-in player has blocked.
    #[serde(default)]
    pub blocked: Vec<UserRef>,
}
