use sqlx::PgExecutor;
use uuid::Uuid;

use crate::{
    enums::puzzles::RushKind,
    models::puzzles::rush::{BestRushRow, NewRushRun},
};

pub async fn insert(db: impl PgExecutor<'_>, run: &NewRushRun) -> Result<(), sqlx::Error> {
    sqlx::query_file!(
        "src/repositories/sql/puzzles/rush_run/insert.sql",
        run.id,
        run.user_id,
        run.kind.key(),
        run.score,
    )
    .execute(db)
    .await?;
    Ok(())
}

/// The player's best score of `kind`, or `None` before their first.
pub async fn best(
    db: impl PgExecutor<'_>,
    user_id: Uuid,
    kind: RushKind,
) -> Result<Option<i32>, sqlx::Error> {
    sqlx::query_file_scalar!(
        "src/repositories/sql/puzzles/rush_run/best.sql",
        user_id,
        kind.key()
    )
    .fetch_one(db)
    .await
}

/// The player's best score of each kind played.
pub async fn bests(
    db: impl PgExecutor<'_>,
    user_id: Uuid,
) -> Result<Vec<BestRushRow>, sqlx::Error> {
    sqlx::query_file_as!(
        BestRushRow,
        "src/repositories/sql/puzzles/rush_run/bests.sql",
        user_id
    )
    .fetch_all(db)
    .await
}
