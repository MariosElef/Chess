use sqlx::PgExecutor;

use crate::models::puzzles::attempt::NewAttempt;

pub async fn insert(db: impl PgExecutor<'_>, attempt: &NewAttempt) -> Result<(), sqlx::Error> {
    sqlx::query_file!(
        "src/repositories/sql/puzzles/attempt/insert.sql",
        attempt.id,
        attempt.user_id,
        attempt.puzzle_id,
        attempt.solved,
        attempt.duration_ms,
        attempt.rated,
    )
    .execute(db)
    .await?;
    Ok(())
}
