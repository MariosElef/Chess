use axum::{
    extract::FromRequestParts,
    http::{header, request::Parts},
};
use uuid::Uuid;

use crate::{
    errors::{AppError, ChessCode},
    state::AppState,
};

/// The signed-in user, from an `Authorization: Bearer <token>` header naming a live session.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SignedIn {
    pub user_id: Uuid,
    pub session_id: Uuid,
    /// One of the shared guest accounts: nothing personal, such as a rating, is kept for it.
    pub guest: bool,
}

impl FromRequestParts<AppState> for SignedIn {
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, state: &AppState) -> Result<Self, AppError> {
        let token = bearer_token(parts).ok_or(ChessCode::SignInRequired)?;
        let session = state.sessions.authenticate(token).await?;
        Ok(Self {
            user_id: session.user_id,
            session_id: session.session_id,
            guest: session.guest,
        })
    }
}

fn bearer_token(parts: &Parts) -> Option<&str> {
    parts
        .headers
        .get(header::AUTHORIZATION)?
        .to_str()
        .ok()?
        .strip_prefix("Bearer ")
}
