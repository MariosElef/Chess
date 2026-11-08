use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::message::ChatMessageModel;
use crate::{enums::chat::ConversationKind, models::social::friend::UserRef};

/// A conversation in the player's list: its name, who is in it, its last message and unread count.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ConversationModel {
    pub id: Uuid,
    pub kind: ConversationKind,
    /// A group's name; for a direct chat, the other player's.
    pub title: String,
    pub members: Vec<UserRef>,
    pub last_message: Option<ChatMessageModel>,
    pub unread: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct CreateDirectModel {
    pub user_id: Uuid,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct CreateGroupModel {
    pub title: String,
    pub member_ids: Vec<Uuid>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct AddMemberModel {
    pub user_id: Uuid,
}

/// A conversation of the player's as stored, with their unread count.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConversationRow {
    pub id: Uuid,
    pub kind: String,
    pub title: Option<String>,
    pub unread: i64,
}

/// A member of one of several conversations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemberRow {
    pub conversation_id: Uuid,
    pub user_id: Uuid,
    pub username: String,
}

/// A conversation to store as it starts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewConversation {
    pub id: Uuid,
    pub kind: ConversationKind,
    pub title: Option<String>,
    pub direct_key: Option<String>,
    pub created_by_id: Option<Uuid>,
}
