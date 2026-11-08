use serde::Deserialize;

use crate::models::social::UserRef;

/// A friend's invitation to a game, waiting for an answer.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Challenge {
    pub id: String,
    pub from: UserRef,
    pub to: UserRef,
    pub initial_s: u32,
    pub increment_s: u32,
    pub rated: bool,
}
