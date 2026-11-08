//! The app's pages.

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Page {
    Play,
    Puzzles,
    Analyze,
    Social,
    Settings,
    Account,
}

impl Page {
    /// The pages listed in the sidebar; Settings and Account open from the profile menu instead.
    pub const NAV: [Self; 4] = [Self::Play, Self::Puzzles, Self::Analyze, Self::Social];

    pub fn label(self) -> &'static str {
        match self {
            Self::Play => "Play",
            Self::Puzzles => "Puzzles",
            Self::Analyze => "Analyze",
            Self::Social => "Social",
            Self::Settings => "Settings",
            Self::Account => "Account",
        }
    }
}
