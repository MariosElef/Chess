use sqlx::{PgConnection, PgExecutor};
use uuid::Uuid;

use crate::models::account::ClosingCheck;

/// Holds the player's row until the closing commits, so nothing is recorded for them meanwhile.
pub async fn lock_user(db: &mut PgConnection, user_id: Uuid) -> Result<(), sqlx::Error> {
    sqlx::query_file_scalar!(
        "src/repositories/sql/account/closure/lock_user.sql",
        user_id
    )
    .fetch_one(db)
    .await?;
    Ok(())
}

/// What closing the account checks first: its password, and whether it is a guest.
pub async fn password_hash(
    db: impl PgExecutor<'_>,
    user_id: Uuid,
) -> Result<Option<ClosingCheck>, sqlx::Error> {
    sqlx::query_file_as!(
        ClosingCheck,
        "src/repositories/sql/account/closure/password_hash.sql",
        user_id
    )
    .fetch_optional(db)
    .await
}

/// Everyone with a friend request waiting to or from the player.
pub async fn request_partners(
    db: impl PgExecutor<'_>,
    user_id: Uuid,
) -> Result<Vec<Uuid>, sqlx::Error> {
    sqlx::query_file_scalar!(
        "src/repositories/sql/account/closure/request_partners.sql",
        user_id
    )
    .fetch_all(db)
    .await
}

/// Everyone who shares a chat with the player.
pub async fn fellow_members(
    db: impl PgExecutor<'_>,
    user_id: Uuid,
) -> Result<Vec<Uuid>, sqlx::Error> {
    sqlx::query_file_scalar!(
        "src/repositories/sql/account/closure/fellow_members.sql",
        user_id
    )
    .fetch_all(db)
    .await
}

/// Groups left without members; a game's chat stays with its game.
pub async fn forget_empty_groups(db: &mut PgConnection) -> Result<(), sqlx::Error> {
    sqlx::query_file!("src/repositories/sql/account/closure/forget_empty_groups.sql")
        .execute(db)
        .await?;
    Ok(())
}

/// Their friendships, either way.
pub async fn forget_friendships(db: &mut PgConnection, user_id: Uuid) -> Result<(), sqlx::Error> {
    sqlx::query_file!(
        "src/repositories/sql/account/closure/forget_friendships.sql",
        user_id
    )
    .execute(db)
    .await?;
    Ok(())
}

/// Their game challenges, sent or received.
pub async fn forget_challenges(db: &mut PgConnection, user_id: Uuid) -> Result<(), sqlx::Error> {
    sqlx::query_file!(
        "src/repositories/sql/account/closure/forget_challenges.sql",
        user_id
    )
    .execute(db)
    .await?;
    Ok(())
}

/// Their direct chats, with everything in them.
pub async fn forget_direct_chats(db: &mut PgConnection, user_id: Uuid) -> Result<(), sqlx::Error> {
    sqlx::query_file!(
        "src/repositories/sql/account/closure/forget_direct_chats.sql",
        user_id
    )
    .execute(db)
    .await?;
    Ok(())
}

/// Every message they sent.
pub async fn forget_messages(db: &mut PgConnection, user_id: Uuid) -> Result<(), sqlx::Error> {
    sqlx::query_file!(
        "src/repositories/sql/account/closure/forget_messages.sql",
        user_id
    )
    .execute(db)
    .await?;
    Ok(())
}

/// Their places in group chats.
pub async fn forget_memberships(db: &mut PgConnection, user_id: Uuid) -> Result<(), sqlx::Error> {
    sqlx::query_file!(
        "src/repositories/sql/account/closure/forget_memberships.sql",
        user_id
    )
    .execute(db)
    .await?;
    Ok(())
}

/// Every conversation they are in or started, held until the transaction ends.
pub async fn held_conversations(
    db: &mut PgConnection,
    user_id: Uuid,
) -> Result<Vec<Uuid>, sqlx::Error> {
    sqlx::query_file_scalar!(
        "src/repositories/sql/account/closure/held_conversations.sql",
        user_id
    )
    .fetch_all(db)
    .await
}

/// Blocks they made and blocks made on them.
pub async fn forget_blocks(db: &mut PgConnection, user_id: Uuid) -> Result<(), sqlx::Error> {
    sqlx::query_file!(
        "src/repositories/sql/account/closure/forget_blocks.sql",
        user_id
    )
    .execute(db)
    .await?;
    Ok(())
}

/// Groups they started, which their other members keep.
pub async fn forget_conversations_started(
    db: &mut PgConnection,
    user_id: Uuid,
) -> Result<(), sqlx::Error> {
    sqlx::query_file!(
        "src/repositories/sql/account/closure/forget_conversations_started.sql",
        user_id
    )
    .execute(db)
    .await?;
    Ok(())
}

/// Their rating history.
pub async fn forget_rating_history(
    db: &mut PgConnection,
    user_id: Uuid,
) -> Result<(), sqlx::Error> {
    sqlx::query_file!(
        "src/repositories/sql/account/closure/forget_rating_history.sql",
        user_id
    )
    .execute(db)
    .await?;
    Ok(())
}

/// Their ratings.
pub async fn forget_ratings(db: &mut PgConnection, user_id: Uuid) -> Result<(), sqlx::Error> {
    sqlx::query_file!(
        "src/repositories/sql/account/closure/forget_ratings.sql",
        user_id
    )
    .execute(db)
    .await?;
    Ok(())
}

/// Their puzzle attempts.
pub async fn forget_puzzle_attempts(
    db: &mut PgConnection,
    user_id: Uuid,
) -> Result<(), sqlx::Error> {
    sqlx::query_file!(
        "src/repositories/sql/account/closure/forget_puzzle_attempts.sql",
        user_id
    )
    .execute(db)
    .await?;
    Ok(())
}

/// Their Puzzle Rush scores.
pub async fn forget_rush_runs(db: &mut PgConnection, user_id: Uuid) -> Result<(), sqlx::Error> {
    sqlx::query_file!(
        "src/repositories/sql/account/closure/forget_rush_runs.sql",
        user_id
    )
    .execute(db)
    .await?;
    Ok(())
}

/// The account itself: scrubbed and marked closed.
pub async fn close(db: &mut PgConnection, user_id: Uuid) -> Result<(), sqlx::Error> {
    sqlx::query_file!("src/repositories/sql/account/closure/close.sql", user_id)
        .execute(db)
        .await?;
    Ok(())
}
