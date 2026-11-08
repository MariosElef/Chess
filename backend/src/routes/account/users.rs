use axum::{
    Json, Router,
    extract::State,
    http::StatusCode,
    routing::{get, post},
};

use crate::{
    errors::AppError,
    extract::{Body, SignedIn},
    models::{
        account::{CreateUserModel, GetUserModel},
        message::CreatedModel,
    },
    state::AppState,
};

pub fn router() -> Router<AppState> {
    Router::new().route("/", post(create)).route("/me", get(me))
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
