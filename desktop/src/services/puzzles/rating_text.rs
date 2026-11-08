use crate::models::puzzles::{PuzzleRating, PuzzleSummary};

/// The menu's line about the puzzle rating: the rating, that there is none yet, or that a
/// guest keeps none.
pub fn rating_line(summary: &PuzzleSummary) -> String {
    match (summary.guest, summary.rating) {
        (true, _) => "Playing as a guest: sign up to keep a rating and best scores.".to_owned(),
        (false, Some(rating)) => format!("Puzzle rating {}", rating_text(rating)),
        (false, None) => "No puzzle rating yet: solve a Practice puzzle to get one.".to_owned(),
    }
}

/// `1512 (+12)` after an attempt, `1512?` while it is unsure.
pub fn rating_with_change(rating: PuzzleRating) -> String {
    match rating.change {
        Some(change) => format!("{} ({change:+})", rating_text(rating)),
        None => rating_text(rating),
    }
}

/// `Best 12`, or nothing played yet.
pub fn best_text(best: Option<i32>) -> String {
    best.map_or_else(|| "No score yet".to_owned(), |best| format!("Best {best}"))
}

fn rating_text(rating: PuzzleRating) -> String {
    let unsure = if rating.provisional { "?" } else { "" };
    format!("{}{unsure}", rating.rating)
}

#[cfg(test)]
mod tests {
    use super::{best_text, rating_line, rating_with_change};
    use crate::models::puzzles::{PuzzleRating, PuzzleSummary};

    fn rating(change: Option<i32>, provisional: bool) -> PuzzleRating {
        PuzzleRating {
            rating: 1512,
            deviation: 80,
            provisional,
            change,
        }
    }

    #[test]
    fn a_rating_reads_with_its_change_and_its_doubt() {
        assert_eq!(rating_with_change(rating(Some(12), false)), "1512 (+12)");
        assert_eq!(rating_with_change(rating(Some(-8), true)), "1512? (-8)");
    }

    #[test]
    fn the_menu_line_says_what_there_is() {
        let guest = PuzzleSummary {
            guest: true,
            ..PuzzleSummary::default()
        };
        assert!(rating_line(&guest).starts_with("Playing as a guest"));
        assert!(rating_line(&PuzzleSummary::default()).starts_with("No puzzle rating yet"));
        let rated = PuzzleSummary {
            rating: Some(rating(None, false)),
            ..PuzzleSummary::default()
        };
        assert_eq!(rating_line(&rated), "Puzzle rating 1512");
        assert_eq!(best_text(Some(12)), "Best 12");
    }
}
