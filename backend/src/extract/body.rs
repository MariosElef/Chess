use axum::{
    Json,
    extract::{FromRequest, Request, rejection::JsonRejection},
};
use serde::de::DeserializeOwned;

use crate::errors::{AppError, ChessCode};

/// A JSON body whose rejection is a problem like every other error, not axum's plain text.
pub struct Body<T>(pub T);

impl<S, T> FromRequest<S> for Body<T>
where
    T: DeserializeOwned,
    S: Send + Sync,
{
    type Rejection = AppError;

    async fn from_request(request: Request, state: &S) -> Result<Self, Self::Rejection> {
        let Json(value) = Json::<T>::from_request(request, state)
            .await
            .map_err(unreadable)?;
        Ok(Self(value))
    }
}

fn unreadable(rejection: JsonRejection) -> AppError {
    AppError::with_detail(ChessCode::InvalidRequestBody, rejection.body_text())
}
