use axum::{Router, extract::State, routing::get};

use crate::{errors::AppError, state::AppState};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/health", get(live))
        .route("/ready", get(ready))
}

/// The process answers.
async fn live() -> &'static str {
    "ok"
}

/// The process can do its work: the database answers too.
async fn ready(State(state): State<AppState>) -> Result<&'static str, AppError> {
    state.health.ready().await?;
    Ok("ready")
}
