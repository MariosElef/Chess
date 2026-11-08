use sqlx::PgExecutor;
use uuid::Uuid;

use crate::models::chat::conversation::{ConversationRow, MemberRow, NewConversation};

/// `user`'s conversations but games' chats, most recently active first.
pub async fn of_user(
    db: impl PgExecutor<'_>,
    user: Uuid,
) -> Result<Vec<ConversationRow>, sqlx::Error> {
    sqlx::query_file_as!(
        ConversationRow,
        "src/repositories/sql/chat/conversation/of_user.sql",
        user
    )
    .fetch_all(db)
    .await
}

/// One conversation of `user`'s; `None` when they are not in it.
pub async fn one_of_user(
    db: impl PgExecutor<'_>,
    user: Uuid,
    conversation: Uuid,
) -> Result<Option<ConversationRow>, sqlx::Error> {
    sqlx::query_file_as!(
        ConversationRow,
        "src/repositories/sql/chat/conversation/one_of_user.sql",
        user,
        conversation
    )
    .fetch_optional(db)
    .await
}

pub async fn members_of(
    db: impl PgExecutor<'_>,
    conversations: &[Uuid],
) -> Result<Vec<MemberRow>, sqlx::Error> {
    sqlx::query_file_as!(
        MemberRow,
        "src/repositories/sql/chat/conversation/members_of.sql",
        conversations
    )
    .fetch_all(db)
    .await
}

pub async fn member_ids(
    db: impl PgExecutor<'_>,
    conversation: Uuid,
) -> Result<Vec<Uuid>, sqlx::Error> {
    sqlx::query_file_scalar!(
        "src/repositories/sql/chat/conversation/member_ids.sql",
        conversation
    )
    .fetch_all(db)
    .await
}

/// The direct conversation with this key, if the pair has one.
pub async fn find_direct(
    db: impl PgExecutor<'_>,
    direct_key: &str,
) -> Result<Option<Uuid>, sqlx::Error> {
    sqlx::query_file_scalar!(
        "src/repositories/sql/chat/conversation/find_direct.sql",
        direct_key
    )
    .fetch_optional(db)
    .await
}

pub async fn insert(
    db: impl PgExecutor<'_>,
    conversation: &NewConversation,
) -> Result<(), sqlx::Error> {
    sqlx::query_file!(
        "src/repositories/sql/chat/conversation/insert.sql",
        conversation.id,
        conversation.kind.key(),
        conversation.title,
        conversation.direct_key,
        conversation.created_by_id
    )
    .execute(db)
    .await?;
    Ok(())
}

/// Adds `user` with `role` (`owner` or `member`); nothing if they are in already.
pub async fn add_member(
    db: impl PgExecutor<'_>,
    conversation: Uuid,
    user: Uuid,
    role: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query_file!(
        "src/repositories/sql/chat/conversation/add_member.sql",
        conversation,
        user,
        role
    )
    .execute(db)
    .await?;
    Ok(())
}

pub async fn remove_member(
    db: impl PgExecutor<'_>,
    conversation: Uuid,
    user: Uuid,
) -> Result<(), sqlx::Error> {
    sqlx::query_file!(
        "src/repositories/sql/chat/conversation/remove_member.sql",
        conversation,
        user
    )
    .execute(db)
    .await?;
    Ok(())
}

/// Everything in the conversation is now read by `user`.
pub async fn mark_read(
    db: impl PgExecutor<'_>,
    conversation: Uuid,
    user: Uuid,
) -> Result<(), sqlx::Error> {
    sqlx::query_file!(
        "src/repositories/sql/chat/conversation/mark_read.sql",
        conversation,
        user
    )
    .execute(db)
    .await?;
    Ok(())
}
