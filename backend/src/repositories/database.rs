use std::time::Duration;

use sqlx::{PgPool, postgres::PgPoolOptions};

use crate::errors::AppError;

/// The connection pool: a cheap handle, cloned into every service.
pub type Db = PgPool;

const MAX_CONNECTIONS: u32 = 10;
/// Waiting longer than this for a connection fails the request instead of hanging it.
const ACQUIRE_TIMEOUT: Duration = Duration::from_secs(5);
const MAX_LIFETIME: Duration = Duration::from_secs(30 * 60);

pub async fn connect(url: &str) -> anyhow::Result<Db> {
    let db = PgPoolOptions::new()
        .max_connections(MAX_CONNECTIONS)
        .acquire_timeout(ACQUIRE_TIMEOUT)
        .max_lifetime(MAX_LIFETIME)
        .connect(url)
        .await?;
    Ok(db)
}

/// Applies the migrations in `backend/migrations` that have not run yet.
pub async fn migrate(db: &Db) -> anyhow::Result<()> {
    sqlx::migrate!().run(db).await?;
    Ok(())
}

/// Whether the database answers, for the readiness probe.
pub async fn ping(db: &Db) -> Result<(), sqlx::Error> {
    sqlx::query_scalar!(r#"SELECT 1 AS "one!""#)
        .fetch_one(db)
        .await?;
    Ok(())
}

/// A database failure is ours, never the client's: logged in full, answered as a 500.
impl From<sqlx::Error> for AppError {
    fn from(error: sqlx::Error) -> Self {
        Self::Unexpected(error.into())
    }
}
