use serde::Deserialize;

/// Another player as the signed-in one stands with them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Relation {
    None,
    Friends,
    /// The signed-in player asked and is waiting.
    RequestSent,
    /// The other player asked; the signed-in one can accept.
    RequestReceived,
}
