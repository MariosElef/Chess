use super::problem::Problem;

/// Why a request to the server did not succeed. It carries the server's problem, so it sits
/// with the models rather than among the plain enums.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ServerError {
    /// The server answered and said no.
    Refused(Problem),
    /// No answer: the server is down, or this computer is offline.
    Unreachable,
    /// An answer this app cannot read.
    Unexpected(String),
}
