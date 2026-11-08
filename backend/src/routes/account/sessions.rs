use axum::{
    Json, Router,
    extract::State,
    http::{HeaderMap, StatusCode, header},
    routing::{delete, post},
};

use crate::{
    errors::AppError,
    extract::{Body, SignedIn},
    models::{
        account::{CreateSessionModel, GetSessionModel},
        message::MessageResponse,
    },
    state::AppState,
};

/// More of a user agent than this is not kept.
const USER_AGENT_MAX: usize = 200;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", post(create))
        .route("/guest", post(create_guest))
        .route("/current", delete(delete_current))
}

/// Signs in; the answer's token goes in `Authorization: Bearer` from then on.
async fn create(
    State(state): State<AppState>,
    headers: HeaderMap,
    Body(body): Body<CreateSessionModel>,
) -> Result<(StatusCode, Json<GetSessionModel>), AppError> {
    let response = state.sessions.sign_in(body, user_agent(&headers)).await?;
    Ok((StatusCode::CREATED, Json(response)))
}

/// Signs in as the first free guest account; signing out frees it for the next guest.
async fn create_guest(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<(StatusCode, Json<GetSessionModel>), AppError> {
    let response = state
        .sessions
        .sign_in_as_guest(user_agent(&headers))
        .await?;
    Ok((StatusCode::CREATED, Json(response)))
}

/// Signs out this session; the user's other devices stay signed in.
async fn delete_current(
    State(state): State<AppState>,
    signed_in: SignedIn,
) -> Result<Json<MessageResponse>, AppError> {
    state.sessions.sign_out(signed_in.session_id).await?;
    let response = MessageResponse {
        message: "Signed out successfully".to_owned(),
    };
    Ok(Json(response))
}

/// Which app signed in, to tell sessions apart later.
fn user_agent(headers: &HeaderMap) -> Option<String> {
    let agent = headers.get(header::USER_AGENT)?.to_str().ok()?;
    Some(agent.chars().take(USER_AGENT_MAX).collect())
}
