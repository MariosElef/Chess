//! How the app looks: light, dark, or following the Mac; the board's colours and its pieces.

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

    pub fn label(self) -> &'static str {
        match self {
            Self::System => "System",
            Self::Light => "Light",
            Self::Dark => "Dark",
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

    pub fn label(self) -> &'static str {
        match self {
            Self::Slate => "Slate",
            Self::Brown => "Brown",
            Self::Blue => "Blue",
            Self::Green => "Green",
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

    pub fn label(self) -> &'static str {
        match self {
            Self::Classic => "Classic",
            Self::Modern => "Modern",
            Self::Staunty => "Staunty",
            Self::California => "California",
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

    pub fn label(self) -> &'static str {
        match self {
            Self::Standard => "Standard",
            Self::Lisp => "Lisp",
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
    use super::{Appearance, BoardTheme, PieceSet, SoundSet};

    #[test]
    fn every_choice_reads_back_from_its_key() {
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
