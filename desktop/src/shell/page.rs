use gpui_kit::assets::IconName;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Page {
    Play,
    Puzzles,
    Analyze,
    Settings,
}

impl Page {
    /// The pages listed in the sidebar; Settings opens from the profile menu instead.
    pub const NAV: [Self; 3] = [Self::Play, Self::Puzzles, Self::Analyze];

    pub fn label(self) -> &'static str {
        match self {
            Self::Play => "Play",
            Self::Puzzles => "Puzzles",
            Self::Analyze => "Analyze",
            Self::Settings => "Settings",
        }
    }

    pub fn icon(self) -> IconName {
        match self {
            Self::Play => IconName::ChessKnight,
            Self::Puzzles => IconName::Puzzle,
            Self::Analyze => IconName::Microscope,
            Self::Settings => IconName::Settings,
        }
    }
}
