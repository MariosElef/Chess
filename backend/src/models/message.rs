use serde::Serialize;
use uuid::Uuid;

/// The answer to a create: the new row's id and what happened.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CreatedModel {
    pub id: Uuid,
    pub message: String,
}

/// The answer to an action that returns nothing else.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct MessageResponse {
    pub message: String,
}
