use serde::Deserialize;

use super::chat_message::ChatMessage;
use crate::{enums::chat::ConversationKind, models::social::UserRef};

/// A conversation in the player's list: its name, members, last message and unread count.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Conversation {
    pub id: String,
    pub kind: ConversationKind,
    pub title: String,
    pub members: Vec<UserRef>,
    /// Who can rename the group and remove others; none for a direct chat.
    #[serde(default)]
    pub owner_id: Option<String>,
    pub last_message: Option<ChatMessage>,
    pub unread: u32,
}
