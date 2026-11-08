use sqlx::PgExecutor;
use uuid::Uuid;

use crate::models::social::friend::{FriendshipRow, PairRow};

/// The friendship of a pair, given smaller id first.
pub async fn between(
    db: impl PgExecutor<'_>,
    (first, second): (Uuid, Uuid),
) -> Result<Option<PairRow>, sqlx::Error> {
    sqlx::query_file_as!(
        PairRow,
        "src/repositories/sql/social/friendship/between.sql",
        first,
        second
    )
    .fetch_optional(db)
    .await
}

/// Asks, or asks again; nothing for a blocked pair. Whether a request now stands.
pub async fn request(
    db: impl PgExecutor<'_>,
    (first, second): (Uuid, Uuid),
    requester: Uuid,
) -> Result<bool, sqlx::Error> {
    let done = sqlx::query_file!(
        "src/repositories/sql/social/friendship/request.sql",
        first,
        second,
        requester
    )
    .execute(db)
    .await?;
    Ok(done.rows_affected() > 0)
}

/// Accepts the request `requester` made; whether there was one.
pub async fn accept(
    db: impl PgExecutor<'_>,
    (first, second): (Uuid, Uuid),
    requester: Uuid,
) -> Result<bool, sqlx::Error> {
    let done = sqlx::query_file!(
        "src/repositories/sql/social/friendship/accept.sql",
        first,
        second,
        requester
    )
    .execute(db)
    .await?;
    Ok(done.rows_affected() > 0)
}

/// Back to strangers; whether there was anything to undo.
pub async fn remove(
    db: impl PgExecutor<'_>,
    (first, second): (Uuid, Uuid),
) -> Result<bool, sqlx::Error> {
    let done = sqlx::query_file!(
        "src/repositories/sql/social/friendship/remove.sql",
        first,
        second
    )
    .execute(db)
    .await?;
    Ok(done.rows_affected() > 0)
}

/// Friends and pending requests of `user`, by name.
pub async fn of_user(
    db: impl PgExecutor<'_>,
    user: Uuid,
) -> Result<Vec<FriendshipRow>, sqlx::Error> {
    sqlx::query_file_as!(
        FriendshipRow,
        "src/repositories/sql/social/friendship/of_user.sql",
        user
    )
    .fetch_all(db)
    .await
}

/// Up to `limit` players matching `pattern`, a lowercase LIKE-escaped piece of a name.
pub async fn search(
    db: impl PgExecutor<'_>,
    user: Uuid,
    pattern: &str,
    limit: i64,
) -> Result<Vec<FriendshipRow>, sqlx::Error> {
    sqlx::query_file_as!(
        FriendshipRow,
        "src/repositories/sql/social/friendship/search.sql",
        user,
        pattern,
        limit
    )
    .fetch_all(db)
    .await
}

pub async fn friend_ids(db: impl PgExecutor<'_>, user: Uuid) -> Result<Vec<Uuid>, sqlx::Error> {
    sqlx::query_file_scalar!(
        "src/repositories/sql/social/friendship/friend_ids.sql",
        user
    )
    .fetch_all(db)
    .await
}
