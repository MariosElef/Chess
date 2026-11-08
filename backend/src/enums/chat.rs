use serde::Serialize;

/// What a conversation is, as `social.conversations.kind` stores it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ConversationKind {
    /// Two friends.
    Direct,
    /// Several friends, with a name.
    Group,
    /// The two players of an online game.
    Game,
}

impl ConversationKind {
    pub fn key(self) -> &'static str {
        match self {
            Self::Direct => "direct",
            Self::Group => "group",
            Self::Game => "game",
        }
    }

    pub fn from_key(key: &str) -> Option<Self> {
        match key {
            "direct" => Some(Self::Direct),
            "group" => Some(Self::Group),
            "game" => Some(Self::Game),
            _ => None,
        }
    }
}
