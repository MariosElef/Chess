use axum::{
    Json, Router,
    extract::{Path, Query, State},
    http::StatusCode,
    routing::{delete, get, post},
};
use uuid::Uuid;

use crate::{
    errors::AppError,
    extract::{Body, SignedIn},
    models::social::friend::{
        CreateFriendRequestModel, FoundUserModel, GetFriendsModel, GetRelationModel,
        UserSearchQuery,
    },
    state::AppState,
};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/search", get(search))
        .route("/friends", get(friends))
        .route("/friends/requests", post(create_request))
        .route("/friends/requests/{user_id}/accept", post(accept_request))
        .route("/friends/{user_id}", delete(remove))
        .route("/blocks/{user_id}", post(block).delete(unblock))
}

/// Players whose name contains `q`, with where the searcher stands with each.
async fn search(
    State(state): State<AppState>,
    signed_in: SignedIn,
    Query(query): Query<UserSearchQuery>,
) -> Result<Json<Vec<FoundUserModel>>, AppError> {
    let response = state.friends.search(signed_in.user_id, &query.q).await?;
    Ok(Json(response))
}

/// Friends, online first, and the requests each way.
async fn friends(
    State(state): State<AppState>,
    signed_in: SignedIn,
) -> Result<Json<GetFriendsModel>, AppError> {
    let response = state.friends.list(signed_in.user_id).await?;
    Ok(Json(response))
}

/// Asks a player to be friends; accepts at once if they had asked first.
async fn create_request(
    State(state): State<AppState>,
    signed_in: SignedIn,
    Body(body): Body<CreateFriendRequestModel>,
) -> Result<(StatusCode, Json<GetRelationModel>), AppError> {
    let relation = state
        .friends
        .request(signed_in.user_id, body.user_id)
        .await?;
    Ok((StatusCode::CREATED, Json(GetRelationModel { relation })))
}

async fn accept_request(
    State(state): State<AppState>,
    signed_in: SignedIn,
    Path(user_id): Path<Uuid>,
) -> Result<StatusCode, AppError> {
    state.friends.accept(signed_in.user_id, user_id).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// Unfriends, declines a request, or withdraws one.
async fn remove(
    State(state): State<AppState>,
    signed_in: SignedIn,
    Path(user_id): Path<Uuid>,
) -> Result<StatusCode, AppError> {
    state.friends.remove(signed_in.user_id, user_id).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// Blocks a player: whatever stood between the two goes, and they cannot ask or message again.
async fn block(
    State(state): State<AppState>,
    signed_in: SignedIn,
    Path(user_id): Path<Uuid>,
) -> Result<StatusCode, AppError> {
    state.friends.block(signed_in.user_id, user_id).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// Lifts a block the player made.
async fn unblock(
    State(state): State<AppState>,
    signed_in: SignedIn,
    Path(user_id): Path<Uuid>,
) -> Result<StatusCode, AppError> {
    state.friends.unblock(signed_in.user_id, user_id).await?;
    Ok(StatusCode::NO_CONTENT)
}
