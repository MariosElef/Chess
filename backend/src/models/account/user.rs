use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// A sign-up. No `Debug`: it carries the password. The email may be left out or blank.
#[derive(Deserialize)]
pub struct CreateUserModel {
    #[serde(default)]
    pub email: Option<String>,
    pub username: String,
    pub password: String,
}

/// An account as its owner sees it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct GetUserModel {
    pub id: Uuid,
    /// None for an account made without one.
    pub email: Option<String>,
    pub username: String,
    pub created_at: DateTime<Utc>,
    /// One of the ten shared guest accounts, Guest to Guest10.
    pub guest: bool,
    /// Their picture's hash, which names it at `/api/avatars/{id}/{hash}`; none without one.
    pub avatar: Option<String>,
}

/// A checked sign-up with its password hashed, ready to store. No `Debug`: the hash.
pub struct NewUser {
    pub id: Uuid,
    pub email: Option<String>,
    pub username: String,
    pub password_hash: String,
}

/// The account a sign-in names, with the hash its password is checked against. No `Debug`.
pub struct UserCredentials {
    pub id: Uuid,
    pub email: Option<String>,
    pub username: String,
    pub created_at: DateTime<Utc>,
    pub guest: bool,
    pub avatar: Option<String>,
    pub password_hash: String,
}

impl UserCredentials {
    /// The account without its hash, for the answer.
    pub fn into_user(self) -> GetUserModel {
        GetUserModel {
            id: self.id,
            email: self.email,
            username: self.username,
            created_at: self.created_at,
            guest: self.guest,
            avatar: self.avatar,
        }
    }
}

/// A picture just set: the hash that names it at `/api/avatars/{id}/{hash}`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct GetAvatarModel {
    pub avatar: String,
}
