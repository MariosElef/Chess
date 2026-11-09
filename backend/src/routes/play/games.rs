use axum::{
    Json, Router,
    extract::{Path, Query, State},
    routing::get,
};
use uuid::Uuid;

use crate::{
    errors::{AppError, ChessCode},
    extract::SignedIn,
    models::play::history::{GameRecordModel, HistoryQuery, PlayedGameModel},
    state::AppState,
};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/games", get(page))
        .route("/games/{id}", get(one))
}

/// A page of the player's finished games, the latest first; none for a shared guest account.
async fn page(
    State(state): State<AppState>,
    signed_in: SignedIn,
    Query(query): Query<HistoryQuery>,
) -> Result<Json<Vec<PlayedGameModel>>, AppError> {
    if signed_in.guest {
        return Ok(Json(Vec::new()));
    }
    Ok(Json(
        state.games.page(signed_in.user_id, query.before).await?,
    ))
}

/// One of the player's finished games, with its moves; a guest's are not theirs to look back on.
async fn one(
    State(state): State<AppState>,
    signed_in: SignedIn,
    Path(id): Path<Uuid>,
) -> Result<Json<GameRecordModel>, AppError> {
    if signed_in.guest {
        return Err(ChessCode::GameNotFound.into());
    }
    Ok(Json(state.games.one(signed_in.user_id, id).await?))
}
