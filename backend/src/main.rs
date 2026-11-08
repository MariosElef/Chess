use std::{env, path::PathBuf};

use anyhow::{Context as _, bail};
use chess_backend::{
    AppConfig, AppState, build_router, import_puzzles, shutdown_signal, telemetry,
};
use tokio::net::TcpListener;

const USAGE: &str = "usage: chess-backend [import-puzzles <lichess_db_puzzle.csv.zst>]";

// No dotenv loader: configuration arrives in the process environment, so there is one source.
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    telemetry::init();
    let config = AppConfig::load()?;
    let mut args = env::args().skip(1);
    match args.next().as_deref() {
        None => serve(&config).await,
        Some("import-puzzles") => {
            let path = args.next().map(PathBuf::from).context(USAGE)?;
            import_puzzles(&config, path).await
        }
        Some(_) => bail!(USAGE),
    }
}

async fn serve(config: &AppConfig) -> anyhow::Result<()> {
    let state = AppState::build(config).await?;
    let app = build_router(state);
    let listener = TcpListener::bind(&config.bind_addr).await?;
    tracing::info!(address = %config.bind_addr, "listening");
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;
    Ok(())
}
