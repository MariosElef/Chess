use reqwest::Response;
use serde::de::DeserializeOwned;

use crate::models::server::ServerError;

pub async fn read_json<T: DeserializeOwned>(response: Response) -> Result<T, ServerError> {
    response
        .json()
        .await
        .map_err(|error| ServerError::Unexpected(error.to_string()))
}

/// The other side is down, or the request timed out: unreachable. Anything else is ours.
pub fn no_answer(error: reqwest::Error) -> ServerError {
    if error.is_connect() || error.is_timeout() {
        ServerError::Unreachable
    } else {
        ServerError::Unexpected(error.to_string())
    }
}
