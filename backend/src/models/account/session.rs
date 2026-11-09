use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::user::GetUserModel;

/// A sign-in by email or username. No `Debug`: it carries the password.
#[derive(Deserialize)]
pub struct CreateSessionModel {
    pub login: String,
    pub password: String,
}

/// A new session: its token, shown this once, when it expires, and whose it is; no `Debug`.
#[derive(Serialize)]
pub struct GetSessionModel {
    pub token: String,
    pub expires_at: DateTime<Utc>,
    pub user: GetUserModel,
}

/// A session to store: the token's hash, never the token. No `Debug`.
pub struct NewSession {
    pub id: Uuid,
    pub user_id: Uuid,
    pub token_hash: Vec<u8>,
    pub user_agent: Option<String>,
}

/// The live session a token names, and its user.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SessionUser {
    pub session_id: Uuid,
    pub user_id: Uuid,
    /// One of the shared guest accounts.
    pub guest: bool,
}

/// A live session as stored, with how long ago it began and was last used.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionRow {
    pub id: Uuid,
    pub user_agent: Option<String>,
    pub signed_in_s: i64,
    pub seen_s: i64,
}

/// A device signed in to the account, as the Devices card lists it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DeviceModel {
    pub id: Uuid,
    /// What signed in, as a person reads it: "Chess app on macOS".
    pub device: String,
    /// The session asking.
    pub current: bool,
    /// Seconds since it signed in, and since it was last used.
    pub signed_in_s: i64,
    pub seen_s: i64,
}
