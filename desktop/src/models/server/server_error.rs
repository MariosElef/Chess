use super::problem::Problem;

/// Why a request to the server did not succeed, carrying the server's problem if it had one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ServerError {
    /// The server answered and said no.
    Refused(Problem),
    /// No answer: the server is down, or this computer is offline.
    Unreachable,
    /// An answer this app cannot read.
    Unexpected(String),
}
