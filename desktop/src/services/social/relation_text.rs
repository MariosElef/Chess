use crate::enums::social::Relation;

/// Where the player stands with someone, in a word or two; none where there is a button instead.
pub fn relation_text(relation: Relation) -> Option<&'static str> {
    match relation {
        Relation::Friends => Some("Friends"),
        Relation::RequestSent => Some("Request sent"),
        Relation::Blocked => Some("Blocked"),
        Relation::None | Relation::RequestReceived => None,
    }
}

#[cfg(test)]
mod tests {
    use super::relation_text;
    use crate::enums::social::Relation;

    #[test]
    fn only_a_settled_relation_is_put_in_words() {
        assert_eq!(relation_text(Relation::Friends), Some("Friends"));
        assert_eq!(relation_text(Relation::RequestSent), Some("Request sent"));
        assert_eq!(relation_text(Relation::None), None, "Add friend instead");
        assert_eq!(
            relation_text(Relation::RequestReceived),
            None,
            "Accept instead"
        );
    }
}
