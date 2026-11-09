//! Which account form is open.

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum AccountForm {
    #[default]
    SignIn,
    SignUp,
}

impl AccountForm {
    pub fn title(self) -> &'static str {
        match self {
            Self::SignIn => "Sign in",
            Self::SignUp => "Create account",
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
    pub fn switch_label(self) -> &'static str {
        match self {
            Self::SignIn => "New here? Create an account",
            Self::SignUp => "Already have an account? Sign in",
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

    pub fn label(self) -> &'static str {
        match self {
            Self::Games => "Games",
            Self::Battles => "Battles",
            Self::Puzzles => "Puzzles",
        }
    }
}
