use serde::Deserialize;

use crate::{enums::social::Relation, models::account::Stats};

/// Another player as anyone signed in may see them: name, picture, ratings and record.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Profile {
    pub id: String,
    pub username: String,
    /// Their picture's hash; none while they show their initial.
    #[serde(default)]
    pub avatar: Option<String>,
    /// When the account was made, as a date such as "2026-11-09".
    pub joined: String,
    /// Where the player stands with them.
    pub relation: Relation,
    /// It is the player's own profile.
    pub mine: bool,
    pub stats: Stats,
}
