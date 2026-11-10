//! Which account form is open.
use crate::i18n::tr;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum AccountForm {
    #[default]
    SignIn,
    SignUp,
}

impl AccountForm {
    pub fn title(self) -> String {
        match self {
            Self::SignIn => tr!("account.form.sign_in"),
            Self::SignUp => tr!("account.form.sign_up"),
        }
    }

    /// The other form, for the link under the button.
    pub fn other(self) -> Self {
        match self {
            Self::SignIn => Self::SignUp,
            Self::SignUp => Self::SignIn,
        }
    }

    /// The link to the other form.
    pub fn switch_label(self) -> String {
        match self {
            Self::SignIn => tr!("account.form.to_sign_up"),
            Self::SignUp => tr!("account.form.to_sign_in"),
        }
    }
}

/// The Account page's history tabs: online games, puzzle battles, puzzle attempts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum HistoryTab {
    #[default]
    Games,
    Battles,
    Puzzles,
}

impl HistoryTab {
    pub const ALL: [Self; 3] = [Self::Games, Self::Battles, Self::Puzzles];

    pub fn label(self) -> String {
        match self {
            Self::Games => tr!("account.history.tabs.games"),
            Self::Battles => tr!("account.history.tabs.battles"),
            Self::Puzzles => tr!("account.history.tabs.puzzles"),
        }
    }

    /// What the tab's element is called, which never changes with the language.
    pub fn id(self) -> &'static str {
        match self {
            Self::Games => "games",
            Self::Battles => "battles",
            Self::Puzzles => "puzzles",
        }
    }
}
