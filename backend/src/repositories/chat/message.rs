use chrono::{DateTime, Utc};
use sqlx::PgExecutor;
use uuid::Uuid;

use crate::models::chat::message::{MessageRow, NewMessage};

/// Stores the message; when it was sent.
pub async fn insert(
    db: impl PgExecutor<'_>,
    message: &NewMessage,
) -> Result<DateTime<Utc>, sqlx::Error> {
    sqlx::query_file_scalar!(
        "src/repositories/sql/chat/message/insert.sql",
        message.id,
        message.conversation_id,
        message.sender_id,
        message.body
    )
    .fetch_one(db)
    .await
}

/// Up to `limit` messages before `before` (the newest without it), the latest first.
pub async fn page(
    db: impl PgExecutor<'_>,
    conversation: Uuid,
    before: Option<Uuid>,
    limit: i64,
) -> Result<Vec<MessageRow>, sqlx::Error> {
    sqlx::query_file_as!(
        MessageRow,
        "src/repositories/sql/chat/message/page.sql",
        conversation,
        before,
        limit
    )
    .fetch_all(db)
    .await
}

/// The last message of each conversation that has one.
pub async fn last_of(
    db: impl PgExecutor<'_>,
    conversations: &[Uuid],
) -> Result<Vec<MessageRow>, sqlx::Error> {
    sqlx::query_file_as!(
        MessageRow,
        "src/repositories/sql/chat/message/last_of.sql",
        conversations
    )
    .fetch_all(db)
    .await
}
