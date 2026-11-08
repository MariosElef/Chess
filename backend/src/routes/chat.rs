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
    models::chat::{
        conversation::{AddMemberModel, ConversationModel, CreateDirectModel, CreateGroupModel},
        message::{ChatMessageModel, CreateMessageModel, MessagesQuery},
    },
    state::AppState,
};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/conversations", get(list))
        .route("/direct", post(open_direct))
        .route("/groups", post(create_group))
        .route("/groups/{id}/members", post(add_member))
        .route("/groups/{id}/members/me", delete(leave))
        .route("/conversations/{id}/messages", get(messages).post(send))
        .route("/conversations/{id}/read", post(mark_read))
}

/// The player's conversations, most recently active first.
async fn list(
    State(state): State<AppState>,
    signed_in: SignedIn,
) -> Result<Json<Vec<ConversationModel>>, AppError> {
    Ok(Json(state.chat.list(signed_in.user_id).await?))
}

/// The conversation with a friend, started the first time.
async fn open_direct(
    State(state): State<AppState>,
    signed_in: SignedIn,
    Body(body): Body<CreateDirectModel>,
) -> Result<Json<ConversationModel>, AppError> {
    let response = state
        .chat
        .open_direct(signed_in.user_id, body.user_id)
        .await?;
    Ok(Json(response))
}

async fn create_group(
    State(state): State<AppState>,
    signed_in: SignedIn,
    Body(body): Body<CreateGroupModel>,
) -> Result<(StatusCode, Json<ConversationModel>), AppError> {
    let response = state
        .chat
        .create_group(signed_in.user_id, &body.title, &body.member_ids)
        .await?;
    Ok((StatusCode::CREATED, Json(response)))
}

async fn add_member(
    State(state): State<AppState>,
    signed_in: SignedIn,
    Path(id): Path<Uuid>,
    Body(body): Body<AddMemberModel>,
) -> Result<Json<ConversationModel>, AppError> {
    let response = state
        .chat
        .add_member(signed_in.user_id, id, body.user_id)
        .await?;
    Ok(Json(response))
}

async fn leave(
    State(state): State<AppState>,
    signed_in: SignedIn,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, AppError> {
    state.chat.leave(signed_in.user_id, id).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// A page of messages, oldest first: the newest, or those before `before`.
async fn messages(
    State(state): State<AppState>,
    signed_in: SignedIn,
    Path(id): Path<Uuid>,
    Query(query): Query<MessagesQuery>,
) -> Result<Json<Vec<ChatMessageModel>>, AppError> {
    let response = state
        .chat
        .messages(signed_in.user_id, id, query.before)
        .await?;
    Ok(Json(response))
}

async fn send(
    State(state): State<AppState>,
    signed_in: SignedIn,
    Path(id): Path<Uuid>,
    Body(body): Body<CreateMessageModel>,
) -> Result<(StatusCode, Json<ChatMessageModel>), AppError> {
    let response = state.chat.send(signed_in.user_id, id, &body.body).await?;
    Ok((StatusCode::CREATED, Json(response)))
}

async fn mark_read(
    State(state): State<AppState>,
    signed_in: SignedIn,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, AppError> {
    state.chat.mark_read(signed_in.user_id, id).await?;
    Ok(StatusCode::NO_CONTENT)
}
