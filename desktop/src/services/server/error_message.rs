use crate::models::server::ServerError;

const UNREACHABLE: &str = "Can't reach the server. Check that it is running.";
const UNEXPECTED: &str = "Something went wrong. Try again.";

/// The sentence a person reads for a failed request: the server's own words, or what went wrong.
pub fn error_message(error: &ServerError) -> String {
    match error {
        ServerError::Refused(problem) => problem.message().to_owned(),
        ServerError::Unreachable => UNREACHABLE.to_owned(),
        ServerError::Unexpected(_) => UNEXPECTED.to_owned(),
    }
}

/// Whether the server refused with this problem `type`, such as `/problems/no-puzzle-in-range`.
pub fn is_problem(error: &ServerError, problem_type: &str) -> bool {
    matches!(error, ServerError::Refused(problem) if problem.problem_type == problem_type)
}

#[cfg(test)]
mod tests {
    use super::{error_message, is_problem};
    use crate::models::server::{Problem, ServerError};

    #[test]
    fn a_refusal_reads_the_servers_words() {
        let refused = ServerError::Refused(Problem {
            problem_type: "/problems/lichess-unavailable".to_owned(),
            title: "Puzzles are unavailable right now".to_owned(),
            detail: None,
            errors: Vec::new(),
        });
        assert_eq!(error_message(&refused), "Puzzles are unavailable right now");
        assert!(is_problem(&refused, "/problems/lichess-unavailable"));
        assert!(!is_problem(
            &ServerError::Unreachable,
            "/problems/lichess-unavailable"
        ));
        assert!(error_message(&ServerError::Unreachable).starts_with("Can't reach"));
    }
}
