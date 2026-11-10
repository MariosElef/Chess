use crate::{i18n::tr, services::time::MINUTE};

const HOUR: i64 = 60 * MINUTE;
const DAY: i64 = 24 * HOUR;
const MONTH: i64 = 30 * DAY;
const YEAR: i64 = 365 * DAY;

/// The line under a friend's name: "Online now" or how long since they were.
pub fn last_online_text(online: bool, away_s: Option<i64>) -> String {
    if online {
        return tr!("social.last_online.online");
    }
    match away_s {
        None => tr!("social.last_online.never"),
        Some(away) if away < MINUTE => tr!("social.last_online.just_now"),
        Some(away) => away_text(away),
    }
}

/// "Last online 3 days ago": the largest whole unit, its noun in the form its count takes.
fn away_text(seconds: i64) -> String {
    if seconds < HOUR {
        let count = seconds / MINUTE;
        return if count == 1 {
            tr!("social.last_online.minutes_one", count = count)
        } else {
            tr!("social.last_online.minutes_other", count = count)
        };
    }
    if seconds < DAY {
        let count = seconds / HOUR;
        return if count == 1 {
            tr!("social.last_online.hours_one", count = count)
        } else {
            tr!("social.last_online.hours_other", count = count)
        };
    }
    if seconds < MONTH {
        let count = seconds / DAY;
        return if count == 1 {
            tr!("social.last_online.days_one", count = count)
        } else {
            tr!("social.last_online.days_other", count = count)
        };
    }
    if seconds < YEAR {
        let count = seconds / MONTH;
        return if count == 1 {
            tr!("social.last_online.months_one", count = count)
        } else {
            tr!("social.last_online.months_other", count = count)
        };
    }
    let count = seconds / YEAR;
    if count == 1 {
        tr!("social.last_online.years_one", count = count)
    } else {
        tr!("social.last_online.years_other", count = count)
    }
}

#[cfg(test)]
mod tests {
    use super::last_online_text;
    use crate::{enums::settings::Language, i18n::set_current};

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

    #[test]
    fn it_speaks_greek_when_greek_is_shown() {
        set_current(Language::Greek);
        let text = last_online_text(false, Some(3 * 86_400));
        set_current(Language::English);
        assert_eq!(text, "Ήταν online πριν από 3 ημέρες");
    }
}
