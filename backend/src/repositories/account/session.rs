use chrono::{DateTime, Utc};
use sqlx::PgExecutor;
use uuid::Uuid;

use crate::models::account::{NewSession, SessionRow, SessionUser};

/// Stores the session, valid for `hours` from the database's now; returns when it ends.
pub async fn insert(
    db: impl PgExecutor<'_>,
    session: &NewSession,
    hours: i32,
) -> Result<DateTime<Utc>, sqlx::Error> {
    sqlx::query_file_scalar!(
        "src/repositories/sql/account/session/insert.sql",
        session.id,
        session.user_id,
        session.token_hash,
        session.user_agent,
        hours,
    )
    .fetch_one(db)
    .await
}

/// The live session with this token hash, marked as just seen; `None` if unknown or ended.
pub async fn authenticate(
    db: impl PgExecutor<'_>,
    token_hash: &[u8],
) -> Result<Option<SessionUser>, sqlx::Error> {
    sqlx::query_file_as!(
        SessionUser,
        "src/repositories/sql/account/session/authenticate.sql",
        token_hash,
    )
    .fetch_optional(db)
    .await
}

/// Ends every session of the user, so a guest handed to someone new is no longer the last holder's.
pub async fn revoke_all_of_user(db: impl PgExecutor<'_>, user_id: Uuid) -> Result<(), sqlx::Error> {
    sqlx::query_file!(
        "src/repositories/sql/account/session/revoke_all_of_user.sql",
        user_id
    )
    .execute(db)
    .await?;
    Ok(())
}

pub async fn revoke(db: impl PgExecutor<'_>, session_id: Uuid) -> Result<(), sqlx::Error> {
    sqlx::query_file!(
        "src/repositories/sql/account/session/revoke.sql",
        session_id
    )
    .execute(db)
    .await?;
    Ok(())
}

/// The user's live sessions, the most recently used first.
pub async fn of_user(
    db: impl PgExecutor<'_>,
    user_id: Uuid,
) -> Result<Vec<SessionRow>, sqlx::Error> {
    sqlx::query_file_as!(
        SessionRow,
        "src/repositories/sql/account/session/of_user.sql",
        user_id
    )
    .fetch_all(db)
    .await
}

/// Ends every session of the user but `keep`.
pub async fn revoke_others(
    db: impl PgExecutor<'_>,
    user_id: Uuid,
    keep: Uuid,
) -> Result<(), sqlx::Error> {
    sqlx::query_file!(
        "src/repositories/sql/account/session/revoke_others.sql",
        user_id,
        keep
    )
    .execute(db)
    .await?;
    Ok(())
}

/// Whether the session is still signed in.
pub async fn is_live(db: impl PgExecutor<'_>, session_id: Uuid) -> Result<bool, sqlx::Error> {
    sqlx::query_file_scalar!(
        "src/repositories/sql/account/session/is_live.sql",
        session_id
    )
    .fetch_one(db)
    .await
}
