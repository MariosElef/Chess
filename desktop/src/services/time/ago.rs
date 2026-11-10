pub const MINUTE: i64 = 60;
const HOUR: i64 = 60 * MINUTE;
const DAY: i64 = 24 * HOUR;
const MONTH: i64 = 30 * DAY;
const YEAR: i64 = 365 * DAY;

use crate::i18n::tr;

/// "just now", or how long ago in the largest whole unit: "3 days ago".
pub fn ago_text(seconds: i64) -> String {
    if seconds < MINUTE {
        return tr!("common.time.just_now");
    }
    tr!("common.time.ago", span = span_words(seconds))
}

/// "5 minutes", "1 hour", "3 days", "2 months", "1 year".
pub fn span_words(seconds: i64) -> String {
    if seconds < HOUR {
        counted("minute", seconds / MINUTE)
    } else if seconds < DAY {
        counted("hour", seconds / HOUR)
    } else if seconds < MONTH {
        counted("day", seconds / DAY)
    } else if seconds < YEAR {
        counted("month", seconds / MONTH)
    } else {
        counted("year", seconds / YEAR)
    }
}

/// A count of one unit in the current language, in its singular or its plural form.
fn counted(unit: &str, count: i64) -> String {
    let form = if count == 1 { "one" } else { "other" };
    tr!(&format!("common.time.{unit}_{form}"), count = count)
}

#[cfg(test)]
mod tests {
    use super::ago_text;
    use crate::{enums::settings::Language, i18n::set_current};

    #[test]
    fn it_says_the_largest_whole_unit() {
        assert_eq!(ago_text(5), "just now");
        assert_eq!(ago_text(90), "1 minute ago");
        assert_eq!(ago_text(3 * 86_400 + 10), "3 days ago");
    }

    #[test]
    fn it_speaks_greek_with_its_plurals() {
        set_current(Language::Greek);
        assert_eq!(ago_text(5), "μόλις τώρα");
        assert_eq!(ago_text(3_600), "πριν από 1 ώρα");
        assert_eq!(ago_text(3 * 86_400 + 10), "πριν από 3 ημέρες");
        set_current(Language::English);
    }
}
