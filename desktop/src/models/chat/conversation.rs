use serde::Deserialize;

use super::chat_message::ChatMessage;
use crate::{enums::chat::ConversationKind, models::social::UserRef};

/// A conversation in the player's list: what it is called for them, who is in it, its last
/// message and how many they have not read.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Conversation {
    pub id: String,
    pub kind: ConversationKind,
    pub title: String,
    pub members: Vec<UserRef>,
    pub last_message: Option<ChatMessage>,
    pub unread: u32,
}
