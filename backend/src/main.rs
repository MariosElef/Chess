use chess_backend::{AppConfig, AppState, build_router, shutdown_signal, telemetry};
use tokio::net::TcpListener;

// No dotenv loader: configuration arrives in the process environment, so there is one source.
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    telemetry::init();
    let config = AppConfig::load()?;
    let state = AppState::build(&config).await?;
    let app = build_router(state);
    let listener = TcpListener::bind(&config.bind_addr).await?;
    tracing::info!(address = %config.bind_addr, "listening");
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;
    Ok(())
}
