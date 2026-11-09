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

/// A change made from the Group modal.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GroupChange {
    /// The owner names the group anew.
    Rename(String),
    /// A member adds this friend of theirs.
    Add(String),
    /// The owner takes this member out.
    Remove(String),
    /// The player leaves.
    Leave,
}
