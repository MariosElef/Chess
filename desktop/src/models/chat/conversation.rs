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
    pub last_message: Option<ChatMessage>,
    pub unread: u32,
}
