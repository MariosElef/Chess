use serde::Serialize;
use uuid::Uuid;

use super::GetStatsModel;
use crate::enums::social::Relation;

/// Another player as anyone signed in may see them: name, picture, ratings and record.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ProfileModel {
    pub id: Uuid,
    pub username: String,
    pub avatar: Option<String>,
    /// When the account was made, as a date such as "2026-11-09".
    pub joined: String,
    /// Where the asker stands with them; `None` too when it is the asker's own profile.
    pub relation: Relation,
    /// It is the asker's own profile.
    pub mine: bool,
    pub stats: GetStatsModel,
}
