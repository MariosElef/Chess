use serde::Deserialize;

/// What a conversation is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConversationKind {
    /// Two friends.
    Direct,
    /// Several friends, with a name.
    Group,
    /// The two players of an online game.
    Game,
}
