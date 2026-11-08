use serde::Deserialize;

use super::user::User;

/// A signed-in session: the token sent with every request, and whose it is. No `Debug`.
#[derive(Clone, PartialEq, Eq, Deserialize)]
pub struct Session {
    pub token: String,
    pub user: User,
}
