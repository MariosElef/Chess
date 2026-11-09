use serde::Serialize;
use uuid::Uuid;

use crate::{enums::play::Color, models::social::friend::UserRef};

/// An invitation to a game, waiting for an answer: a friend's challenge, or a rematch.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ChallengeModel {
    pub id: Uuid,
    pub from: UserRef,
    pub to: UserRef,
    pub initial_s: u32,
    pub increment_s: u32,
    pub rated: bool,
    /// The finished game this asks to play again, with colours swapped; none for a challenge.
    pub rematch_of: Option<Uuid>,
}

/// A challenge as the lists show it, with how long it waits and how much of that is left now.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ListedChallenge {
    #[serde(flatten)]
    pub challenge: ChallengeModel,
    /// How long it waits in all.
    pub wait_s: u32,
    /// How much of the wait is left as it is sent.
    pub seconds_left: u32,
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
    /// The challenger's colour for a rematch; a friend's challenge leaves it to chance.
    pub challenger_colour: Option<Color>,
    pub rematch_of: Option<Uuid>,
    /// How long it waits for an answer.
    pub expires_in_s: i32,
}
