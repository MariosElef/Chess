mod avatars;
mod sessions;
mod users;

use axum::Router;

use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        .nest("/users", users::router())
        .nest("/sessions", sessions::router())
}

/// Profile pictures, with their own body limit: a picture is larger than anything else sent.
pub fn avatar_router() -> Router<AppState> {
    avatars::router()
}
