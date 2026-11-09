use serde::Deserialize;

/// An account as the server shows it to its owner.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct User {
    pub id: String,
    pub email: String,
    pub username: String,
    /// One of the ten shared guest accounts, Guest to Guest10.
    #[serde(default)]
    pub guest: bool,
    /// Their picture's hash; none while they show their initial.
    #[serde(default)]
    pub avatar: Option<String>,
}
