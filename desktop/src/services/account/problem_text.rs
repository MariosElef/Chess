use crate::{
    i18n::tr,
    models::{
        account::AccountProblem,
        server::{Problem, ServerError},
    },
};

/// The sentence for a problem, in the language being shown.
pub fn problem_text(problem: &AccountProblem) -> String {
    match problem {
        AccountProblem::Server(error) => server_text(error),
        AccountProblem::SignedOut => tr!("account.problems.signed_out"),
        AccountProblem::InvalidEmail => tr!("account.problems.invalid_email"),
        AccountProblem::UsernameFormat => tr!("account.problems.username_format"),
        AccountProblem::PasswordLength => tr!("account.problems.password_length"),
        AccountProblem::PictureTooLarge => tr!("account.problems.picture_too_large"),
        AccountProblem::PictureUnreadable => tr!("account.problems.picture_unreadable"),
        AccountProblem::GameUnreadable => tr!("account.problems.game_unreadable"),
    }
}

fn server_text(error: &ServerError) -> String {
    match error {
        ServerError::Refused(problem) => refusal_text(problem),
        ServerError::Unreachable => tr!("account.problems.unreachable"),
        ServerError::Unexpected(_) => tr!("account.problems.unexpected"),
    }
}

/// A refusal by its code; one this app has no words for shows the server's own.
fn refusal_text(problem: &Problem) -> String {
    match problem.problem_type.as_str() {
        "/problems/invalid-credentials" => tr!("account.problems.server.invalid_credentials"),
        "/problems/email-taken" => tr!("account.problems.server.email_taken"),
        "/problems/username-taken" => tr!("account.problems.server.username_taken"),
        "/problems/all-guests-busy" => tr!("account.problems.server.all_guests_busy"),
        "/problems/sign-in-required" => tr!("account.problems.server.sign_in_required"),
        "/problems/invalid-fields" => tr!("account.problems.server.invalid_fields"),
        "/problems/invalid-request-body" => tr!("account.problems.server.invalid_request_body"),
        "/problems/route-not-found" => tr!("account.problems.server.route_not_found"),
        "/problems/guest-cannot-close" => tr!("account.problems.server.guest_cannot_close"),
        "/problems/guest-cannot-have-avatar" => {
            tr!("account.problems.server.guest_cannot_have_avatar")
        }
        "/problems/avatar-unreadable" => tr!("account.problems.server.avatar_unreadable"),
        "/problems/avatar-too-large" => tr!("account.problems.server.avatar_too_large"),
        "/problems/avatar-not-found" => tr!("account.problems.server.avatar_not_found"),
        "/problems/internal-error" => tr!("account.problems.server.internal_error"),
        _ => problem.message().to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::problem_text;
    use crate::{
        enums::settings::Language,
        i18n::set_current,
        models::{
            account::AccountProblem,
            server::{Problem, ServerError},
        },
    };

    fn refused(problem_type: &str) -> AccountProblem {
        AccountProblem::Server(ServerError::Refused(Problem {
            problem_type: problem_type.to_owned(),
            title: "The server's title".to_owned(),
            detail: None,
            errors: Vec::new(),
        }))
    }

    #[test]
    fn a_code_is_worded_in_the_language_shown_and_an_unknown_one_keeps_the_servers_words() {
        let wrong = refused("/problems/invalid-credentials");
        assert_eq!(problem_text(&wrong), "Wrong email, username or password");
        assert_eq!(
            problem_text(&refused("/problems/something-new")),
            "The server's title"
        );
        set_current(Language::Greek);
        assert_eq!(problem_text(&wrong), "Λάθος email, όνομα χρήστη ή κωδικός");
        set_current(Language::English);
    }
}
