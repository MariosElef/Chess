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
    /// The signed-in player blocked them.
    Blocked,
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

/// A leaderboard of everyone's ratings: one per rating category.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum RankingBoard {
    Bullet,
    #[default]
    Blitz,
    Rapid,
    Classical,
    Puzzles,
    PuzzleBattle,
}

impl RankingBoard {
    pub const ALL: [Self; 6] = [
        Self::Bullet,
        Self::Blitz,
        Self::Rapid,
        Self::Classical,
        Self::Puzzles,
        Self::PuzzleBattle,
    ];

    /// The board's name in the server's address.
    pub fn key(self) -> &'static str {
        match self {
            Self::Bullet => "bullet",
            Self::Blitz => "blitz",
            Self::Rapid => "rapid",
            Self::Classical => "classical",
            Self::Puzzles => "puzzle",
            Self::PuzzleBattle => "puzzle_battle",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Bullet => "Bullet",
            Self::Blitz => "Blitz",
            Self::Rapid => "Rapid",
            Self::Classical => "Classical",
            Self::Puzzles => "Puzzles",
            Self::PuzzleBattle => "Puzzle Battle",
        }
    }

    /// What a player's count of rated results on this board counts.
    pub fn played_noun(self) -> &'static str {
        match self {
            Self::Bullet | Self::Blitz | Self::Rapid | Self::Classical => "games",
            Self::Puzzles => "puzzles",
            Self::PuzzleBattle => "battles",
        }
    }
}
