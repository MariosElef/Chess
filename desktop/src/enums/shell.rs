//! The app's pages.

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Page {
    Play,
    Puzzles,
    Analyze,
    Social,
    Messages,
    Leaderboard,
    Settings,
    Account,
}

impl Page {
    /// The pages on the rail; Settings and Account open from the avatar menu instead.
    pub const NAV: [Self; 4] = [Self::Play, Self::Puzzles, Self::Analyze, Self::Social];

    /// The pages a rail item opens onto when hovered; empty for an item that is one page.
    pub fn menu(self) -> &'static [Self] {
        match self {
            Self::Social => &[Self::Social, Self::Messages, Self::Leaderboard],
            Self::Play | Self::Puzzles | Self::Analyze | Self::Messages | Self::Leaderboard => &[],
            Self::Settings | Self::Account => &[],
        }
    }

    /// The rail item a page is under: Messages and Leaderboard are under Social.
    pub fn rail_item(self) -> Self {
        match self {
            Self::Messages | Self::Leaderboard => Self::Social,
            Self::Play | Self::Puzzles | Self::Analyze | Self::Social => self,
            Self::Settings | Self::Account => self,
        }
    }

    /// The page's name in its rail item's menu.
    pub fn menu_label(self) -> &'static str {
        match self {
            Self::Social => "Friends",
            Self::Play
            | Self::Puzzles
            | Self::Analyze
            | Self::Messages
            | Self::Leaderboard
            | Self::Settings
            | Self::Account => self.label(),
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Play => "Play",
            Self::Puzzles => "Puzzles",
            Self::Analyze => "Analyze",
            Self::Social => "Social",
            Self::Messages => "Messages",
            Self::Leaderboard => "Leaderboard",
            Self::Settings => "Settings",
            Self::Account => "Account",
        }
    }
}
