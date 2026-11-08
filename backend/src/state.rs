use crate::{
    config::AppConfig,
    repositories::database::{self, Db},
    services::{
        account::{SessionService, UserService},
        health::HealthService,
    },
};

/// The services every handler can reach, built once on one pool.
#[derive(Clone)]
pub struct AppState {
    pub users: UserService,
    pub sessions: SessionService,
    pub health: HealthService,
}

impl AppState {
    /// Connects and migrates the database: a server that cannot reach it does not start.
    pub async fn build(config: &AppConfig) -> anyhow::Result<Self> {
        let db = database::connect(&config.database_url).await?;
        database::migrate(&db).await?;
        Ok(Self::assemble(db))
    }

    /// The services on a pool that is ready, as the tests build them.
    pub fn assemble(db: Db) -> Self {
        Self {
            users: UserService::new(db.clone()),
            sessions: SessionService::new(db.clone()),
            health: HealthService::new(db),
        }
    }
}
