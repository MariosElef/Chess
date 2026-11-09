use sqlx::PgExecutor;
use uuid::Uuid;

use crate::{
    enums::play::{ChallengeStatus, Color},
    models::play::challenge::NewChallenge,
};

pub async fn insert(db: impl PgExecutor<'_>, challenge: &NewChallenge) -> Result<(), sqlx::Error> {
    sqlx::query_file!(
        "src/repositories/sql/play/challenge/insert.sql",
        challenge.id,
        challenge.challenger_id,
        challenge.challenged_id,
        challenge.initial_s,
        challenge.increment_s,
        challenge.rated,
        challenge.challenger_colour.map_or("random", Color::key),
        challenge.rematch_of,
        f64::from(challenge.expires_in_s),
    )
    .execute(db)
    .await?;
    Ok(())
}

/// Settles it; `game_id` only with `Accepted`.
pub async fn resolve(
    db: impl PgExecutor<'_>,
    challenge_id: Uuid,
    status: ChallengeStatus,
    game_id: Option<Uuid>,
) -> Result<(), sqlx::Error> {
    sqlx::query_file!(
        "src/repositories/sql/play/challenge/resolve.sql",
        challenge_id,
        status.key(),
        game_id
    )
    .execute(db)
    .await?;
    Ok(())
}
