use std::time::Duration;

use axum::{
    Json, Router,
    body::Bytes,
    extract::DefaultBodyLimit,
    extract::{Path, State},
    http::{StatusCode, header},
    response::IntoResponse,
    routing::{get, put},
};
use tower_http::{limit::RequestBodyLimitLayer, timeout::TimeoutLayer};
use uuid::Uuid;

use crate::{
    errors::AppError, extract::SignedIn, models::account::GetAvatarModel,
    services::account::stored_format, state::AppState,
};

/// The largest picture taken; the server keeps a small square of it.
const PICTURE_LIMIT_BYTES: usize = 5 * 1024 * 1024;
/// A picture may take a while to come up a slow connection, longer than other requests may.
const UPLOAD_TIMEOUT: Duration = Duration::from_secs(90);
/// Reading a picture takes no longer than any other request.
const READ_TIMEOUT: Duration = Duration::from_secs(30);
/// A picture's URL names its bytes, so whoever has it can keep it for good.
const FOREVER: &str = "public, max-age=31536000, immutable";

/// Setting one's picture, with a larger body than the rest of the API takes, and reading anyone's.
pub fn router() -> Router<AppState> {
    Router::new()
        .route("/users/me/avatar", put(set).delete(remove))
        .layer(RequestBodyLimitLayer::new(PICTURE_LIMIT_BYTES))
        // Axum's extractors cap a body at 2 MB of their own; a picture may be larger.
        .layer(DefaultBodyLimit::max(PICTURE_LIMIT_BYTES))
        .layer(TimeoutLayer::with_status_code(
            StatusCode::REQUEST_TIMEOUT,
            UPLOAD_TIMEOUT,
        ))
        .route(
            "/avatars/{user_id}/{hash}",
            get(image).layer(TimeoutLayer::with_status_code(
                StatusCode::REQUEST_TIMEOUT,
                READ_TIMEOUT,
            )),
        )
}

/// The body, a JPEG, PNG, WebP or GIF, becomes the player's picture.
async fn set(
    State(state): State<AppState>,
    signed_in: SignedIn,
    body: Bytes,
) -> Result<Json<GetAvatarModel>, AppError> {
    let avatar = state
        .avatars
        .set((signed_in.user_id, signed_in.guest), body.to_vec())
        .await?;
    Ok(Json(GetAvatarModel { avatar }))
}

/// The player goes back to their initial.
async fn remove(
    State(state): State<AppState>,
    signed_in: SignedIn,
) -> Result<StatusCode, AppError> {
    state.avatars.remove(signed_in.user_id).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// A picture, cached for good: a new picture has a new hash and so a new URL.
async fn image(
    State(state): State<AppState>,
    Path((user_id, hash)): Path<(Uuid, String)>,
) -> Result<impl IntoResponse, AppError> {
    let image = state.avatars.image(user_id, &hash).await?;
    let headers = [
        (header::CONTENT_TYPE, stored_format().to_mime_type()),
        (header::CACHE_CONTROL, FOREVER),
    ];
    Ok((headers, image))
}
