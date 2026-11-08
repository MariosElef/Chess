use axum::{
    Json, Router,
    extract::{Path, Query, State},
    http::StatusCode,
    routing::{get, post},
};

use crate::{
    errors::AppError,
    extract::{Body, SignedIn},
    models::puzzles::{
        attempt::{CreateAttemptModel, GetAttemptModel},
        puzzle::{GetPuzzleModel, PuzzleQuery},
        rush::{CreateRushModel, GetRushModel},
        summary::GetPuzzleSummaryModel,
    },
    state::AppState,
};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/next", get(next))
        .route("/summary", get(summary))
        .route("/{id}/attempts", post(create_attempt))
        .route("/rush", post(create_rush))
}

/// A puzzle the player has not attempted, rated within `min_rating..=max_rating` when given.
async fn next(
    State(state): State<AppState>,
    signed_in: SignedIn,
    Query(query): Query<PuzzleQuery>,
) -> Result<Json<GetPuzzleModel>, AppError> {
    let response = state.puzzles.next(query, signed_in.user_id).await?;
    Ok(Json(response))
}

/// The puzzle rating, the best rushes ever and today, and the battle rating, for the Puzzles menu.
async fn summary(
    State(state): State<AppState>,
    signed_in: SignedIn,
) -> Result<Json<GetPuzzleSummaryModel>, AppError> {
    let response = state
        .puzzles
        .summary(signed_in.user_id, signed_in.guest)
        .await?;
    Ok(Json(response))
}

/// How a puzzle went; the answer carries the new rating after a rated one.
async fn create_attempt(
    State(state): State<AppState>,
    signed_in: SignedIn,
    Path(puzzle_id): Path<String>,
    Body(body): Body<CreateAttemptModel>,
) -> Result<(StatusCode, Json<GetAttemptModel>), AppError> {
    let response = state
        .attempts
        .record(signed_in.user_id, signed_in.guest, puzzle_id, body)
        .await?;
    Ok((StatusCode::CREATED, Json(response)))
}

/// A finished Puzzle Rush; the answer says whether it is a new best.
async fn create_rush(
    State(state): State<AppState>,
    signed_in: SignedIn,
    Body(body): Body<CreateRushModel>,
) -> Result<(StatusCode, Json<GetRushModel>), AppError> {
    let response = state
        .rushes
        .record(signed_in.user_id, signed_in.guest, body)
        .await?;
    Ok((StatusCode::CREATED, Json(response)))
}
