use serde::Serialize;

/// Where a friendship stands, as `social.friendships.status` stores it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FriendshipStatus {
    Pending,
    Accepted,
    Declined,
    Blocked,
}

impl FriendshipStatus {
    pub fn from_key(key: &str) -> Option<Self> {
        match key {
            "pending" => Some(Self::Pending),
            "accepted" => Some(Self::Accepted),
            "declined" => Some(Self::Declined),
            "blocked" => Some(Self::Blocked),
            _ => None,
        }
    }
}

/// Another player as the signed-in one stands with them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Relation {
    None,
    Friends,
    /// The signed-in player asked and is waiting.
    RequestSent,
    /// The other player asked; the signed-in one can accept.
    RequestReceived,
}
