//! How the app looks and speaks: its language, light or dark or following the Mac, the board's
//! colours and its pieces.
use crate::i18n::tr;

/// The language every word of the app is shown in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Language {
    #[default]
    English,
    Greek,
}

impl Language {
    pub const ALL: [Self; 2] = [Self::English, Self::Greek];

    /// Its place in `ALL`, which is the order the settings page lists the choices.
    pub fn index(self) -> usize {
        match self {
            Self::English => 0,
            Self::Greek => 1,
        }
    }

    /// The language's own name for itself, which is never translated.
    pub fn label(self) -> &'static str {
        match self {
            Self::English => "English",
            Self::Greek => "Ελληνικά",
        }
    }

    /// The locale the translation files are named by.
    pub fn code(self) -> &'static str {
        match self {
            Self::English => "en",
            Self::Greek => "el",
        }
    }

    /// How the choice is written in the settings file.
    pub fn key(self) -> &'static str {
        self.code()
    }

    pub fn from_key(key: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|language| language.key() == key)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Appearance {
    #[default]
    System,
    Light,
    Dark,
}

impl Appearance {
    pub const ALL: [Self; 3] = [Self::System, Self::Light, Self::Dark];

    /// Its place in `ALL`, which is the order the settings page lists the choices.
    pub fn index(self) -> usize {
        match self {
            Self::System => 0,
            Self::Light => 1,
            Self::Dark => 2,
        }
    }

    pub fn label(self) -> String {
        match self {
            Self::System => tr!("settings.appearance.system"),
            Self::Light => tr!("settings.appearance.light"),
            Self::Dark => tr!("settings.appearance.dark"),
        }
    }

    /// How the choice is written in the settings file.
    pub fn key(self) -> &'static str {
        match self {
            Self::System => "system",
            Self::Light => "light",
            Self::Dark => "dark",
        }
    }

    pub fn from_key(key: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|appearance| appearance.key() == key)
    }
}

/// The colours of the board's squares.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum BoardTheme {
    /// The app's own blue-grey, with a darker version in dark mode.
    #[default]
    Slate,
    Brown,
    Blue,
    Green,
}

impl BoardTheme {
    pub const ALL: [Self; 4] = [Self::Slate, Self::Brown, Self::Blue, Self::Green];

    /// Its place in `ALL`, which is the order the settings page lists the choices.
    pub fn index(self) -> usize {
        match self {
            Self::Slate => 0,
            Self::Brown => 1,
            Self::Blue => 2,
            Self::Green => 3,
        }
    }

    pub fn label(self) -> String {
        match self {
            Self::Slate => tr!("settings.board.themes.slate"),
            Self::Brown => tr!("settings.board.themes.brown"),
            Self::Blue => tr!("settings.board.themes.blue"),
            Self::Green => tr!("settings.board.themes.green"),
        }
    }

    /// How the choice is written in the settings file.
    pub fn key(self) -> &'static str {
        match self {
            Self::Slate => "slate",
            Self::Brown => "brown",
            Self::Blue => "blue",
            Self::Green => "green",
        }
    }

    pub fn from_key(key: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|theme| theme.key() == key)
    }
}

/// The drawings used for the pieces.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum PieceSet {
    /// Colin M.L. Burnett's set, the one Lichess uses.
    #[default]
    Classic,
    Modern,
    /// sadsnake1's Staunton-style set, from Lichess.
    Staunty,
    /// Jerry S.'s set, from Lichess.
    California,
}

impl PieceSet {
    pub const ALL: [Self; 4] = [Self::Classic, Self::Modern, Self::Staunty, Self::California];

    /// Its place in `ALL`, which is the order the settings page lists the choices.
    pub fn index(self) -> usize {
        match self {
            Self::Classic => 0,
            Self::Modern => 1,
            Self::Staunty => 2,
            Self::California => 3,
        }
    }

    pub fn label(self) -> String {
        match self {
            Self::Classic => tr!("settings.board.sets.classic"),
            Self::Modern => tr!("settings.board.sets.modern"),
            Self::Staunty => tr!("settings.board.sets.staunty"),
            Self::California => tr!("settings.board.sets.california"),
        }
    }

    /// How the choice is written in the settings file.
    pub fn key(self) -> &'static str {
        match self {
            Self::Classic => "classic",
            Self::Modern => "modern",
            Self::Staunty => "staunty",
            Self::California => "california",
        }
    }

    pub fn from_key(key: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|set| set.key() == key)
    }
}

/// The sounds the app makes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum SoundSet {
    /// chess.com's sounds, the app's first set: personal use only, see CREDITS.md.
    #[default]
    Standard,
    /// EdinburghCollective's set, from Lichess.
    Lisp,
}

impl SoundSet {
    pub const ALL: [Self; 2] = [Self::Standard, Self::Lisp];

    /// Its place in `ALL`, which is the order the settings page lists the choices.
    pub fn index(self) -> usize {
        match self {
            Self::Standard => 0,
            Self::Lisp => 1,
        }
    }

    pub fn label(self) -> String {
        match self {
            Self::Standard => tr!("settings.sounds.sets.standard"),
            Self::Lisp => tr!("settings.sounds.sets.lisp"),
        }
    }

    /// How the choice is written in the settings file.
    pub fn key(self) -> &'static str {
        match self {
            Self::Standard => "standard",
            Self::Lisp => "lisp",
        }
    }

    pub fn from_key(key: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|set| set.key() == key)
    }
}

#[cfg(test)]
mod tests {
    use super::{Appearance, BoardTheme, Language, PieceSet, SoundSet};

    #[test]
    fn every_choice_reads_back_from_its_key() {
        for language in Language::ALL {
            assert_eq!(Language::from_key(language.key()), Some(language));
        }
        for appearance in Appearance::ALL {
            assert_eq!(Appearance::from_key(appearance.key()), Some(appearance));
        }
        for theme in BoardTheme::ALL {
            assert_eq!(BoardTheme::from_key(theme.key()), Some(theme));
        }
        for set in PieceSet::ALL {
            assert_eq!(PieceSet::from_key(set.key()), Some(set));
        }
        for set in SoundSet::ALL {
            assert_eq!(SoundSet::from_key(set.key()), Some(set));
        }
        assert_eq!(BoardTheme::from_key("purple"), None);
    }

    #[test]
    fn every_choice_sits_at_its_index() {
        for (index, language) in Language::ALL.into_iter().enumerate() {
            assert_eq!(language.index(), index);
        }
        for (index, theme) in BoardTheme::ALL.into_iter().enumerate() {
            assert_eq!(theme.index(), index);
        }
        for (index, set) in PieceSet::ALL.into_iter().enumerate() {
            assert_eq!(set.index(), index);
        }
        for (index, set) in SoundSet::ALL.into_iter().enumerate() {
            assert_eq!(set.index(), index);
        }
    }
}
