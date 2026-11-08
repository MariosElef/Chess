use crate::{
    clients::lichess::LichessClient,
    config::AppConfig,
    repositories::{
        database::{self, Db},
        play::game,
    },
    services::{
        account::{SessionService, UserService},
        health::HealthService,
        play::PlayHub,
        puzzles::{attempt::AttemptService, puzzle::PuzzleService, rush::RushService},
        social::FriendService,
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
    pub play: PlayHub,
    pub friends: FriendService,
}

impl AppState {
    /// Connects and migrates the database: a server that cannot reach it does not start.
    pub async fn build(config: &AppConfig) -> anyhow::Result<Self> {
        let db = database::connect(&config.database_url).await?;
        database::migrate(&db).await?;
        let aborted = game::abort_unfinished(&db).await?;
        if aborted > 0 {
            tracing::info!(
                aborted,
                "games left running by the last server closed as aborted"
            );
        }
        let lichess = LichessClient::new(&config.lichess_url)?;
        Ok(Self::assemble(db, lichess))
    }

    /// The services on a pool that is ready and a Lichess to call, as the tests build them.
    pub fn assemble(db: Db, lichess: LichessClient) -> Self {
        let play = PlayHub::new(db.clone());
        Self {
            users: UserService::new(db.clone()),
            sessions: SessionService::new(db.clone(), play.clone()),
            friends: FriendService::new(db.clone(), play.clone()),
            puzzles: PuzzleService::new(db.clone(), lichess),
            attempts: AttemptService::new(db.clone()),
            rushes: RushService::new(db.clone()),
            play,
            health: HealthService::new(db),
        }
    }
}
