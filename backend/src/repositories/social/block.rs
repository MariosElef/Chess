use sqlx::PgExecutor;
use uuid::Uuid;

use crate::models::social::friend::UserRef;

/// `blocker` blocks `blocked`; blocking again changes nothing.
pub async fn insert(
    db: impl PgExecutor<'_>,
    blocker: Uuid,
    blocked: Uuid,
) -> Result<(), sqlx::Error> {
    sqlx::query_file!(
        "src/repositories/sql/social/block/insert.sql",
        blocker,
        blocked
    )
    .execute(db)
    .await?;
    Ok(())
}

/// Lifts `blocker`'s block on `blocked`; whether there was one.
pub async fn delete(
    db: impl PgExecutor<'_>,
    blocker: Uuid,
    blocked: Uuid,
) -> Result<bool, sqlx::Error> {
    let done = sqlx::query_file!(
        "src/repositories/sql/social/block/delete.sql",
        blocker,
        blocked
    )
    .execute(db)
    .await?;
    Ok(done.rows_affected() > 0)
}

/// Everyone `blocker` has blocked.
pub async fn of_blocker(
    db: impl PgExecutor<'_>,
    blocker: Uuid,
) -> Result<Vec<UserRef>, sqlx::Error> {
    sqlx::query_file_as!(
        UserRef,
        "src/repositories/sql/social/block/of_blocker.sql",
        blocker
    )
    .fetch_all(db)
    .await
}

/// Which of the two has blocked the other.
pub async fn blockers_between(
    db: impl PgExecutor<'_>,
    first: Uuid,
    second: Uuid,
) -> Result<Vec<Uuid>, sqlx::Error> {
    sqlx::query_file_scalar!(
        "src/repositories/sql/social/block/blockers_between.sql",
        first,
        second
    )
    .fetch_all(db)
    .await
}

/// Whether anyone among `players` has blocked another of them.
pub async fn any_among(db: impl PgExecutor<'_>, players: &[Uuid]) -> Result<bool, sqlx::Error> {
    sqlx::query_file_scalar!("src/repositories/sql/social/block/any_among.sql", players)
        .fetch_one(db)
        .await
}

/// Whether `player` and any of `others` has blocked the other.
pub async fn with_any(
    db: impl PgExecutor<'_>,
    player: Uuid,
    others: &[Uuid],
) -> Result<bool, sqlx::Error> {
    sqlx::query_file_scalar!(
        "src/repositories/sql/social/block/with_any.sql",
        player,
        others
    )
    .fetch_one(db)
    .await
}
