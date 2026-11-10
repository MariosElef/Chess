use crate::{
    i18n::tr,
    models::server::{Problem, ServerError},
    services::server::error_message,
};

/// The sentence a person reads for a failed friends or chat request: the refusal's code in the
/// language shown, or, for one this page has no words for, whatever `error_message` says.
pub fn problem_text(error: &ServerError) -> String {
    match error {
        ServerError::Refused(problem) => {
            coded_text(problem).unwrap_or_else(|| error_message(error))
        }
        ServerError::Unreachable | ServerError::Unexpected(_) => error_message(error),
    }
}

/// The words for the refusals of friends and chat, found by the code the server answered with.
fn coded_text(problem: &Problem) -> Option<String> {
    let text = match problem.problem_type.strip_prefix("/problems/")? {
        "user-not-found" => tr!("social.errors.user_not_found"),
        "no-friend-request" => tr!("social.errors.no_friend_request"),
        "cannot-befriend-yourself" => tr!("social.errors.cannot_befriend_yourself"),
        "conversation-not-found" => tr!("chat.errors.conversation_not_found"),
        "game-not-found" => tr!("chat.errors.game_not_found"),
        "only-friends" => tr!("chat.errors.only_friends"),
        "not-a-group" => tr!("chat.errors.not_a_group"),
        "not-group-owner" => tr!("chat.errors.not_group_owner"),
        "cannot-join-group" => tr!("chat.errors.cannot_join_group"),
        "message-length" => tr!("chat.errors.message_length"),
        "group-title" => tr!("chat.errors.group_title"),
        _ => return None,
    };
    Some(text)
}

#[cfg(test)]
mod tests {
    use super::problem_text;
    use crate::{
        enums::settings::Language,
        i18n::set_current,
        models::server::{Problem, ServerError},
    };

    fn refused(slug: &str, title: &str) -> ServerError {
        ServerError::Refused(Problem {
            problem_type: format!("/problems/{slug}"),
            title: title.to_owned(),
            detail: None,
            errors: Vec::new(),
        })
    }

    #[test]
    fn a_known_code_reads_in_the_language_shown_and_an_unknown_one_as_the_server_says() {
        let known = refused("only-friends", "server words");
        assert_eq!(problem_text(&known), "You can only chat with friends");
        set_current(Language::Greek);
        let greek = problem_text(&known);
        set_current(Language::English);
        assert_eq!(greek, "Μπορείς να συνομιλείς μόνο με φίλους");
        assert_eq!(
            problem_text(&refused("something-new", "Brand new")),
            "Brand new"
        );
    }
}
