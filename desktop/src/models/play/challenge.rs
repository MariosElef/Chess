use serde::Deserialize;

use crate::models::social::UserRef;

/// An invitation to a game, waiting for an answer: a friend's challenge, or a rematch.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Challenge {
    pub id: String,
    pub from: UserRef,
    pub to: UserRef,
    pub initial_s: u32,
    pub increment_s: u32,
    pub rated: bool,
    /// The finished game it asks to play again, colours swapped; none for a friend's challenge.
    #[serde(default)]
    pub rematch_of: Option<String>,
    /// How long it waits in all.
    pub wait_s: u32,
    /// How much of the wait was left as the server sent it.
    pub seconds_left: u32,
}

impl Challenge {
    /// A rematch, which the game's card and the rematch toast show rather than Social.
    pub fn is_rematch(&self) -> bool {
        self.rematch_of.is_some()
    }
}
