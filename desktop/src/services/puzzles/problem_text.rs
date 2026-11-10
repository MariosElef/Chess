use crate::{i18n::tr, models::puzzles::PuzzleProblem, services::server::error_message};

/// What the page says about a puzzle that could not be had, in the language now shown.
pub fn problem_text(problem: &PuzzleProblem) -> String {
    match problem {
        PuzzleProblem::SignInFirst => tr!("puzzles.sign_in_first"),
        PuzzleProblem::Server(error) => error_message(error),
    }
}
