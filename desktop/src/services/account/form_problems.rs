use crate::models::{
    account::{AccountProblem, FormProblems},
    server::{Problem, ServerError},
};

/// Where each of the server's complaints shows on an account form.
pub fn form_problems(error: &ServerError) -> FormProblems {
    match error {
        ServerError::Refused(problem) => refused(error, problem),
        ServerError::Unreachable | ServerError::Unexpected(_) => form_wide(error),
    }
}

/// Each field error under its field; a taken email or username there; anything else above.
fn refused(error: &ServerError, problem: &Problem) -> FormProblems {
    let mut problems = FormProblems::default();
    for field in &problem.errors {
        if let Some((slot, fault)) = field_slot(&mut problems, &field.pointer) {
            *slot = Some(fault);
        }
    }
    match problem.problem_type.as_str() {
        "/problems/email-taken" => problems.email = Some(AccountProblem::Server(error.clone())),
        "/problems/username-taken" => {
            problems.username = Some(AccountProblem::Server(error.clone()));
        }
        _ if problem.errors.is_empty() => {
            problems.form = Some(AccountProblem::Server(error.clone()));
        }
        _ => {}
    }
    problems
}

/// The slot a field's complaint goes in, and what that field's complaint is.
fn field_slot<'a>(
    problems: &'a mut FormProblems,
    pointer: &str,
) -> Option<(&'a mut Option<AccountProblem>, AccountProblem)> {
    match pointer {
        "/login" => Some((&mut problems.login, AccountProblem::InvalidEmail)),
        "/email" => Some((&mut problems.email, AccountProblem::InvalidEmail)),
        "/username" => Some((&mut problems.username, AccountProblem::UsernameFormat)),
        "/password" => Some((&mut problems.password, AccountProblem::PasswordLength)),
        _ => None,
    }
}

fn form_wide(error: &ServerError) -> FormProblems {
    FormProblems {
        form: Some(AccountProblem::Server(error.clone())),
        ..FormProblems::default()
    }
}

#[cfg(test)]
mod tests {
    use super::form_problems;
    use crate::{
        models::{
            account::AccountProblem,
            server::{FieldProblem, Problem, ServerError},
        },
        services::account::problem_text,
    };

    fn problem(problem_type: &str, title: &str, errors: Vec<FieldProblem>) -> ServerError {
        ServerError::Refused(Problem {
            problem_type: problem_type.to_owned(),
            title: title.to_owned(),
            detail: None,
            errors,
        })
    }

    #[test]
    fn field_errors_go_under_their_fields() {
        let error = problem(
            "/problems/invalid-fields",
            "Some fields are not valid",
            vec![
                FieldProblem {
                    pointer: "/email".to_owned(),
                    detail: "'x' is not an email address".to_owned(),
                },
                FieldProblem {
                    pointer: "/password".to_owned(),
                    detail: "Too short".to_owned(),
                },
            ],
        );
        let problems = form_problems(&error);
        assert_eq!(problems.email, Some(AccountProblem::InvalidEmail));
        assert_eq!(problems.password, Some(AccountProblem::PasswordLength));
        assert_eq!(problems.form, None, "the fields say it all");
    }

    #[test]
    fn a_taken_username_shows_under_the_username() {
        let problems = form_problems(&problem(
            "/problems/username-taken",
            "Username already taken",
            Vec::new(),
        ));
        assert_eq!(
            problems.username.as_ref().map(problem_text),
            Some("Username already taken".to_owned())
        );
        assert_eq!(problems.form, None);
    }

    #[test]
    fn wrong_credentials_and_an_unreachable_server_show_above_the_button() {
        let wrong = form_problems(&problem(
            "/problems/invalid-credentials",
            "Wrong email, username or password",
            Vec::new(),
        ));
        assert_eq!(
            wrong.form.as_ref().map(problem_text),
            Some("Wrong email, username or password".to_owned())
        );
        assert!(form_problems(&ServerError::Unreachable).form.is_some());
    }
}
