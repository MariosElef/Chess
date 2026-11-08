use serde::Deserialize;

/// Closing the account: the password, typed again to be sure.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct CloseAccountModel {
    pub password: String,
}

/// What closing an account checks first.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClosingCheck {
    pub password_hash: String,
    pub guest: bool,
}
