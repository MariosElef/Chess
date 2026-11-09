use serde::Deserialize;

/// A device signed in to the account, as the server lists it.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Device {
    pub id: String,
    /// What signed in, as a person reads it: "Chess app on macOS".
    pub device: String,
    /// This app's own session.
    pub current: bool,
    /// Seconds since it signed in.
    pub signed_in_s: i64,
    /// Seconds since it was last used.
    pub seen_s: i64,
}
