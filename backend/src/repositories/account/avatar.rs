use sqlx::PgExecutor;
use uuid::Uuid;

/// Stores the player's picture under its hash, replacing any earlier one.
pub async fn save(
    db: impl PgExecutor<'_>,
    user_id: Uuid,
    (hash, image): (&str, &[u8]),
) -> Result<(), sqlx::Error> {
    sqlx::query_file!(
        "src/repositories/sql/account/avatar/save.sql",
        user_id,
        hash,
        image
    )
    .execute(db)
    .await?;
    Ok(())
}

/// The player's picture, if `hash` still names it.
pub async fn image(
    db: impl PgExecutor<'_>,
    user_id: Uuid,
    hash: &str,
) -> Result<Option<Vec<u8>>, sqlx::Error> {
    sqlx::query_file_scalar!(
        "src/repositories/sql/account/avatar/image.sql",
        user_id,
        hash
    )
    .fetch_optional(db)
    .await
}

/// The player has no picture any more.
pub async fn delete(db: impl PgExecutor<'_>, user_id: Uuid) -> Result<(), sqlx::Error> {
    sqlx::query_file!("src/repositories/sql/account/avatar/delete.sql", user_id)
        .execute(db)
        .await?;
    Ok(())
}

/// Hands out `hash` as the player's picture from now on; none without one.
pub async fn set_hash(
    db: impl PgExecutor<'_>,
    user_id: Uuid,
    hash: Option<&str>,
) -> Result<(), sqlx::Error> {
    sqlx::query_file!(
        "src/repositories/sql/account/avatar/set_hash.sql",
        user_id,
        hash
    )
    .execute(db)
    .await?;
    Ok(())
}

/// Holds the player's account until the transaction ends; whether it is still open.
pub async fn lock_owner(db: impl PgExecutor<'_>, user_id: Uuid) -> Result<bool, sqlx::Error> {
    let found = sqlx::query_file_scalar!(
        "src/repositories/sql/account/avatar/lock_owner.sql",
        user_id
    )
    .fetch_optional(db)
    .await?;
    Ok(found.is_some())
}
