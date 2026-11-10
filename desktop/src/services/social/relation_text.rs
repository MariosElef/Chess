use crate::{enums::social::Relation, i18n::tr};

/// Where the player stands with someone, in a word or two; none where there is a button instead.
pub fn relation_text(relation: Relation) -> Option<String> {
    match relation {
        Relation::Friends => Some(tr!("social.detail.friends")),
        Relation::RequestSent => Some(tr!("social.detail.request_sent")),
        Relation::Blocked => Some(tr!("social.detail.blocked")),
        Relation::None | Relation::RequestReceived => None,
    }
}

#[cfg(test)]
mod tests {
    use super::relation_text;
    use crate::{
        enums::{settings::Language, social::Relation},
        i18n::set_current,
    };

    #[test]
    fn only_a_settled_relation_is_put_in_words() {
        assert_eq!(relation_text(Relation::Friends).as_deref(), Some("Friends"));
        assert_eq!(
            relation_text(Relation::RequestSent).as_deref(),
            Some("Request sent")
        );
        assert_eq!(relation_text(Relation::None), None, "Add friend instead");
        assert_eq!(
            relation_text(Relation::RequestReceived),
            None,
            "Accept instead"
        );
    }

    #[test]
    fn the_words_follow_the_language_shown() {
        set_current(Language::Greek);
        let text = relation_text(Relation::Friends);
        set_current(Language::English);
        assert_eq!(text.as_deref(), Some("Φίλοι"));
    }
}
