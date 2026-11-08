use serde::Serialize;
use uuid::Uuid;

use crate::models::social::friend::UserRef;

/// A friend's invitation to a game, waiting for an answer.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ChallengeModel {
    pub id: Uuid,
    pub from: UserRef,
    pub to: UserRef,
    pub initial_s: u32,
    pub increment_s: u32,
    pub rated: bool,
}

/// A challenge to store as it is sent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewChallenge {
    pub id: Uuid,
    pub challenger_id: Uuid,
    pub challenged_id: Uuid,
    pub initial_s: i32,
    pub increment_s: i32,
    pub rated: bool,
    /// How long it waits for an answer.
    pub expires_in_s: i32,
}
