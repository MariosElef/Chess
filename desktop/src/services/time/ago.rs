pub const MINUTE: i64 = 60;
const HOUR: i64 = 60 * MINUTE;
const DAY: i64 = 24 * HOUR;
const MONTH: i64 = 30 * DAY;
const YEAR: i64 = 365 * DAY;

/// "just now", or how long ago in the largest whole unit: "3 days ago".
pub fn ago_text(seconds: i64) -> String {
    if seconds < MINUTE {
        return "just now".to_owned();
    }
    format!("{} ago", span_words(seconds))
}

/// "5 minutes", "1 hour", "3 days", "2 months", "1 year".
pub fn span_words(seconds: i64) -> String {
    let (count, unit) = if seconds < HOUR {
        (seconds / MINUTE, "minute")
    } else if seconds < DAY {
        (seconds / HOUR, "hour")
    } else if seconds < MONTH {
        (seconds / DAY, "day")
    } else if seconds < YEAR {
        (seconds / MONTH, "month")
    } else {
        (seconds / YEAR, "year")
    };
    let plural = if count == 1 { "" } else { "s" };
    format!("{count} {unit}{plural}")
}

#[cfg(test)]
mod tests {
    use super::ago_text;

    #[test]
    fn it_says_the_largest_whole_unit() {
        assert_eq!(ago_text(5), "just now");
        assert_eq!(ago_text(90), "1 minute ago");
        assert_eq!(ago_text(3 * 86_400 + 10), "3 days ago");
    }
}
