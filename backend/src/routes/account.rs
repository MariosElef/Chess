mod sessions;
mod users;

use axum::Router;

use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        .nest("/users", users::router())
        .nest("/sessions", sessions::router())
}
