//! The speed bands of a time control.

/// The speed bands ratings are kept for; the same boundaries as the database's `rating_categories`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Category {
    Bullet,
    Blitz,
    Rapid,
    Classical,
}

impl Category {
    pub const ALL: [Self; 4] = [Self::Bullet, Self::Blitz, Self::Rapid, Self::Classical];

    pub fn name(self) -> &'static str {
        match self {
            Self::Bullet => "Bullet",
            Self::Blitz => "Blitz",
            Self::Rapid => "Rapid",
            Self::Classical => "Classical",
        }
    }
}
