const MINUTE: i64 = 60;
const HOUR: i64 = 60 * MINUTE;
const DAY: i64 = 24 * HOUR;
const MONTH: i64 = 30 * DAY;
const YEAR: i64 = 365 * DAY;

/// The line under a friend's name: "Online now" or how long since they were.
pub fn last_online_text(online: bool, away_s: Option<i64>) -> String {
    if online {
        return "Online now".to_owned();
    }
    match away_s {
        None => "Not online yet".to_owned(),
        Some(away) if away < MINUTE => "Last online just now".to_owned(),
        Some(away) => format!("Last online {} ago", span_words(away)),
    }
}

/// "5 minutes", "1 hour", "3 days", "2 months", "1 year".
fn span_words(seconds: i64) -> String {
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
    use super::last_online_text;

    #[test]
    fn it_says_how_long_ago_as_chess_com_does() {
        assert_eq!(last_online_text(true, Some(500)), "Online now");
        assert_eq!(last_online_text(false, None), "Not online yet");
        assert_eq!(last_online_text(false, Some(20)), "Last online just now");
        assert_eq!(
            last_online_text(false, Some(60)),
            "Last online 1 minute ago"
        );
        assert_eq!(
            last_online_text(false, Some(2 * 3600 + 5)),
            "Last online 2 hours ago"
        );
        assert_eq!(
            last_online_text(false, Some(3 * 86_400)),
            "Last online 3 days ago"
        );
        assert_eq!(
            last_online_text(false, Some(400 * 86_400)),
            "Last online 1 year ago"
        );
    }
}
