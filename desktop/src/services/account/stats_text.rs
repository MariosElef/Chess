//! The stats as the Account page words them.
use chess_domain::Category;

/// The game mode a server key names: `blitz` is Blitz.
pub fn category_of(key: &str) -> Option<Category> {
    Category::ALL
        .into_iter()
        .find(|category| category.name().eq_ignore_ascii_case(key))
}

/// `1521`, `1500?` while it is provisional, or a dash before the first rated result.
pub fn rating_text(rating: Option<i32>, provisional: bool) -> String {
    match rating {
        Some(rating) if provisional => format!("{rating}?"),
        Some(rating) => rating.to_string(),
        None => "—".to_owned(),
    }
}

/// The share of puzzles solved, in whole percent; `None` before the first.
pub fn solved_share(solved: u32, attempted: u32) -> Option<u32> {
    (attempted > 0).then(|| solved * 100 / attempted)
}

#[cfg(test)]
mod tests {
    use chess_domain::Category;

    use super::{category_of, rating_text, solved_share};

    #[test]
    fn the_stats_read_as_words() {
        assert_eq!(category_of("blitz"), Some(Category::Blitz));
        assert_eq!(category_of("puzzle"), None);
        assert_eq!(rating_text(Some(1521), false), "1521");
        assert_eq!(rating_text(Some(1500), true), "1500?");
        assert_eq!(rating_text(None, false), "—");
        assert_eq!(solved_share(3, 4), Some(75));
        assert_eq!(solved_share(0, 0), None);
    }
}
