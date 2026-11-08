use serde::Deserialize;

use crate::models::social::UserRef;

/// A message in a conversation, and who sent it.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct ChatMessage {
    pub id: String,
    pub conversation_id: String,
    pub sender: UserRef,
    pub body: String,
}
