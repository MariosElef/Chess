use chess_domain::Category;
use serde::Deserialize;

use crate::i18n::tr;

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

    pub fn label(self) -> String {
        match self {
            Self::RecentlyOnline => tr!("social.sort.recently_online"),
            Self::Username => tr!("social.sort.username"),
        }
    }

    /// What keeps a menu entry apart from the others in whatever language it is shown.
    pub fn key(self) -> &'static str {
        match self {
            Self::RecentlyOnline => "recently-online",
            Self::Username => "username",
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

    pub fn label(self) -> String {
        match self {
            Self::Speed(Category::Bullet) => tr!("social.leaderboard.boards.bullet"),
            Self::Speed(Category::Blitz) => tr!("social.leaderboard.boards.blitz"),
            Self::Speed(Category::Rapid) => tr!("social.leaderboard.boards.rapid"),
            Self::Speed(Category::Classical) => tr!("social.leaderboard.boards.classical"),
            Self::Puzzles => tr!("social.leaderboard.boards.puzzles"),
        }
    }
}

/// A leaderboard of everyone's ratings: one per rating category, and one of each player's average
/// over every category they have played.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum RankingBoard {
    #[default]
    Average,
    Bullet,
    Blitz,
    Rapid,
    Classical,
    Puzzles,
    PuzzleBattle,
}

impl RankingBoard {
    /// In the order the tabs show them, the first being where the leaderboard opens.
    pub const ALL: [Self; 7] = [
        Self::Average,
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
            Self::Average => "average",
        }
    }

    pub fn label(self) -> String {
        match self {
            Self::Bullet => tr!("social.leaderboard.boards.bullet"),
            Self::Blitz => tr!("social.leaderboard.boards.blitz"),
            Self::Rapid => tr!("social.leaderboard.boards.rapid"),
            Self::Classical => tr!("social.leaderboard.boards.classical"),
            Self::Puzzles => tr!("social.leaderboard.boards.puzzles"),
            Self::PuzzleBattle => tr!("social.leaderboard.boards.puzzle_battle"),
            Self::Average => tr!("social.leaderboard.boards.average"),
        }
    }

    /// What a player's count of rated results on this board says, such as "12 games".
    pub fn played_text(self, count: i32) -> String {
        let plural = count != 1;
        match (self, plural) {
            (Self::Bullet | Self::Blitz | Self::Rapid | Self::Classical, false) => {
                tr!("social.leaderboard.played.games_one", count = count)
            }
            (Self::Bullet | Self::Blitz | Self::Rapid | Self::Classical, true) => {
                tr!("social.leaderboard.played.games_other", count = count)
            }
            (Self::Puzzles, false) => tr!("social.leaderboard.played.puzzles_one", count = count),
            (Self::Puzzles, true) => tr!("social.leaderboard.played.puzzles_other", count = count),
            (Self::PuzzleBattle, false) => {
                tr!("social.leaderboard.played.battles_one", count = count)
            }
            (Self::PuzzleBattle, true) => {
                tr!("social.leaderboard.played.battles_other", count = count)
            }
            (Self::Average, false) => tr!("social.leaderboard.played.results_one", count = count),
            (Self::Average, true) => tr!("social.leaderboard.played.results_other", count = count),
        }
    }
}
