use axum::{
    Json, Router,
    extract::{Path, Query, State},
    routing::get,
};

use crate::{
    enums::play::RatingBoard,
    errors::{AppError, ChessCode},
    extract::SignedIn,
    models::rating::{LeaderboardModel, LeaderboardQuery},
    state::AppState,
};

pub fn router() -> Router<AppState> {
    Router::new().route("/leaderboard/{board}", get(board))
}

/// A page of the board for one rating category, with the asker's own place.
async fn board(
    State(state): State<AppState>,
    signed_in: SignedIn,
    Path(board): Path<String>,
    Query(query): Query<LeaderboardQuery>,
) -> Result<Json<LeaderboardModel>, AppError> {
    let board = RatingBoard::from_key(&board).ok_or(ChessCode::RouteNotFound)?;
    let asker = (signed_in.user_id, signed_in.guest);
    Ok(Json(
        state.leaderboard.board(board, asker, query.offset).await?,
    ))
}
