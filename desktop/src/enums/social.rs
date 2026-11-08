use chess_domain::Category;
use serde::Deserialize;

/// Another player as the signed-in one stands with them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Relation {
    None,
    Friends,
    /// The signed-in player asked and is waiting.
    RequestSent,
    /// The other player asked; the signed-in one can accept.
    RequestReceived,
}

/// What the Friends page shows: the friends, or the requests each way.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum SocialStage {
    #[default]
    Friends,
    Requests,
}

/// How the friends are listed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum FriendSort {
    #[default]
    RecentlyOnline,
    Username,
}

impl FriendSort {
    pub const ALL: [Self; 2] = [Self::RecentlyOnline, Self::Username];

    pub fn label(self) -> &'static str {
        match self {
            Self::RecentlyOnline => "Most Recently Online",
            Self::Username => "Username",
        }
    }
}

/// The friends' leaderboard's boards: one per speed, and puzzles.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LeaderBoard {
    Speed(Category),
    Puzzles,
}

impl LeaderBoard {
    pub const ALL: [Self; 5] = [
        Self::Speed(Category::Bullet),
        Self::Speed(Category::Blitz),
        Self::Speed(Category::Rapid),
        Self::Speed(Category::Classical),
        Self::Puzzles,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::Speed(category) => category.name(),
            Self::Puzzles => "Puzzles",
        }
    }
}
