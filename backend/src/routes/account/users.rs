use axum::{
    Json, Router,
    extract::{Path, State},
    http::StatusCode,
    routing::{get, post},
};

use uuid::Uuid;

use crate::{
    enums::social::Relation,
    errors::AppError,
    extract::{Body, SignedIn},
    models::{
        account::{CloseAccountModel, CreateUserModel, GetStatsModel, GetUserModel, ProfileModel},
        message::CreatedModel,
    },
    state::AppState,
};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", post(create))
        .route("/me", get(me).delete(close))
        .route("/me/stats", get(stats))
        .route("/{id}/profile", get(profile))
}

/// Signs up; signing in is a separate request.
async fn create(
    State(state): State<AppState>,
    Body(body): Body<CreateUserModel>,
) -> Result<(StatusCode, Json<CreatedModel>), AppError> {
    let id = state.users.register(body).await?;
    let response = CreatedModel {
        id,
        message: "Account created successfully".to_owned(),
    };
    Ok((StatusCode::CREATED, Json(response)))
}

async fn me(
    State(state): State<AppState>,
    signed_in: SignedIn,
) -> Result<Json<GetUserModel>, AppError> {
    let response = state.users.get(signed_in.user_id).await?;
    Ok(Json(response))
}

/// The player's ratings in every mode and in puzzles, with their results.
async fn stats(
    State(state): State<AppState>,
    signed_in: SignedIn,
) -> Result<Json<GetStatsModel>, AppError> {
    Ok(Json(state.stats.of(signed_in.user_id).await?))
}

/// Another player's profile: name, picture, when they joined, where the asker stands, and stats.
async fn profile(
    State(state): State<AppState>,
    signed_in: SignedIn,
    Path(id): Path<Uuid>,
) -> Result<Json<ProfileModel>, AppError> {
    let player = state.users.player(id).await?;
    let mine = id == signed_in.user_id;
    let relation = if mine {
        Relation::None
    } else {
        state.friends.relation_with(signed_in.user_id, id).await?
    };
    Ok(Json(ProfileModel {
        id,
        username: player.username,
        avatar: player.avatar,
        joined: player.created_at.date_naive().to_string(),
        relation,
        mine,
        stats: state.stats.of(id).await?,
    }))
}

/// Closes the signed-in account, once its password is typed again.
async fn close(
    State(state): State<AppState>,
    signed_in: SignedIn,
    Body(body): Body<CloseAccountModel>,
) -> Result<StatusCode, AppError> {
    state.users.close(signed_in.user_id, body.password).await?;
    Ok(StatusCode::NO_CONTENT)
}
