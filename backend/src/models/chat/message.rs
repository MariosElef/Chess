use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::models::social::friend::UserRef;

/// A message as members see it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ChatMessageModel {
    pub id: Uuid,
    pub conversation_id: Uuid,
    pub sender: UserRef,
    pub body: String,
    pub sent_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct CreateMessageModel {
    pub body: String,
}

/// A page of messages: those before `before`, the latest first; the newest page without it.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct MessagesQuery {
    pub before: Option<Uuid>,
}

/// A message as stored, with its sender's name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MessageRow {
    pub id: Uuid,
    pub conversation_id: Uuid,
    pub sender_id: Uuid,
    pub sender_name: String,
    pub sender_avatar: Option<String>,
    pub body: String,
    pub sent_at: DateTime<Utc>,
}

/// A message to store.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewMessage {
    pub id: Uuid,
    pub conversation_id: Uuid,
    pub sender_id: Uuid,
    pub body: String,
}
