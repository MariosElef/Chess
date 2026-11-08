use crate::{
    models::{
        account::FormProblems,
        server::{Problem, ServerError},
    },
    services::server::error_message,
};

/// Where each of the server's complaints shows on an account form.
pub fn form_problems(error: &ServerError) -> FormProblems {
    match error {
        ServerError::Refused(problem) => refused(problem),
        ServerError::Unreachable | ServerError::Unexpected(_) => form_wide(&error_message(error)),
    }
}

/// Each field error under its field; a taken email or username under that field; anything
/// else above the button.
fn refused(problem: &Problem) -> FormProblems {
    let mut problems = FormProblems::default();
    for field in &problem.errors {
        if let Some(slot) = field_slot(&mut problems, &field.pointer) {
            *slot = Some(field.detail.clone());
        }
    }
    match problem.problem_type.as_str() {
        "/problems/email-taken" => problems.email = Some(problem.title.clone()),
        "/problems/username-taken" => problems.username = Some(problem.title.clone()),
        _ if problem.errors.is_empty() => problems.form = Some(problem.message().to_owned()),
        _ => {}
    }
    problems
}

fn field_slot<'a>(problems: &'a mut FormProblems, pointer: &str) -> Option<&'a mut Option<String>> {
    match pointer {
        "/login" => Some(&mut problems.login),
        "/email" => Some(&mut problems.email),
        "/username" => Some(&mut problems.username),
        "/password" => Some(&mut problems.password),
        _ => None,
    }
}

fn form_wide(message: &str) -> FormProblems {
    FormProblems {
        form: Some(message.to_owned()),
        ..FormProblems::default()
    }
}

#[cfg(test)]
mod tests {
    use super::form_problems;
    use crate::models::server::{FieldProblem, Problem, ServerError};

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
        assert_eq!(
            problems.email.as_deref(),
            Some("'x' is not an email address")
        );
        assert_eq!(problems.password.as_deref(), Some("Too short"));
        assert_eq!(problems.form, None, "the fields say it all");
    }

    #[test]
    fn a_taken_username_shows_under_the_username() {
        let problems = form_problems(&problem(
            "/problems/username-taken",
            "Username already taken",
            Vec::new(),
        ));
        assert_eq!(problems.username.as_deref(), Some("Username already taken"));
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
            wrong.form.as_deref(),
            Some("Wrong email, username or password")
        );
        assert!(form_problems(&ServerError::Unreachable).form.is_some());
    }
}
