use serde::Deserialize;

use super::chat_message::ChatMessage;
use crate::{enums::chat::ConversationKind, i18n::tr, models::social::UserRef};

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

impl Conversation {
    /// What the player reads as its name: a game's chat is named here, in the language shown,
    /// not by the server.
    pub fn shown_title(&self) -> String {
        match self.kind {
            ConversationKind::Game => tr!("chat.game_title"),
            ConversationKind::Direct | ConversationKind::Group => self.title.clone(),
        }
    }

    /// The title a chat view keeps for itself: empty for a game's chat, which that view names
    /// in the language shown whenever it draws.
    pub fn title_to_keep(&self) -> String {
        match self.kind {
            ConversationKind::Game => String::new(),
            ConversationKind::Direct | ConversationKind::Group => self.title.clone(),
        }
    }
}
