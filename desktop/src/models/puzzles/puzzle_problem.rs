use crate::models::server::ServerError;

/// Why a puzzle could not be had: kept as the reason, so the page words it in the language
/// shown when it draws.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PuzzleProblem {
    /// Nobody is signed in.
    SignInFirst,
    /// The server refused, or could not be reached.
    Server(ServerError),
}
