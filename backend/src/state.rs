use crate::{
    clients::lichess::LichessClient,
    config::AppConfig,
    repositories::database::{self, Db},
    services::{
        account::{SessionService, UserService},
        health::HealthService,
        puzzles::{attempt::AttemptService, puzzle::PuzzleService, rush::RushService},
    },
};

/// The services every handler can reach, built once on one pool.
#[derive(Clone)]
pub struct AppState {
    pub users: UserService,
    pub sessions: SessionService,
    pub health: HealthService,
    pub puzzles: PuzzleService,
    pub attempts: AttemptService,
    pub rushes: RushService,
}

impl AppState {
    /// Connects and migrates the database: a server that cannot reach it does not start.
    pub async fn build(config: &AppConfig) -> anyhow::Result<Self> {
        let db = database::connect(&config.database_url).await?;
        database::migrate(&db).await?;
        let lichess = LichessClient::new(&config.lichess_url)?;
        Ok(Self::assemble(db, lichess))
    }

    /// The services on a pool that is ready and a Lichess to call, as the tests build them.
    pub fn assemble(db: Db, lichess: LichessClient) -> Self {
        Self {
            users: UserService::new(db.clone()),
            sessions: SessionService::new(db.clone()),
            puzzles: PuzzleService::new(db.clone(), lichess),
            attempts: AttemptService::new(db.clone()),
            rushes: RushService::new(db.clone()),
            health: HealthService::new(db),
        }
    }
}
