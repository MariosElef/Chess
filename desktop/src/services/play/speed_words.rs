use chess_domain::{Category, TimeControl};

use crate::i18n::tr;

/// A speed's name for players, in the language being shown: `Blitz`.
pub fn category_name(category: Category) -> String {
    match category {
        Category::Bullet => tr!("play.speed.bullet"),
        Category::Blitz => tr!("play.speed.blitz"),
        Category::Rapid => tr!("play.speed.rapid"),
        Category::Classical => tr!("play.speed.classical"),
    }
}

/// A time control for players: `3 | 2` with an increment, `10 min` without.
pub fn control_label(control: TimeControl) -> String {
    let increment = control.increment.as_secs();
    if increment == 0 {
        tr!(
            "play.time.minutes",
            minutes = control.initial.as_secs() / 60
        )
    } else {
        control.label()
    }
}

#[cfg(test)]
mod tests {
    use chess_domain::TimeControl;

    use super::control_label;
    use crate::{enums::settings::Language, i18n::set_current};

    #[test]
    fn minutes_are_said_in_the_language_being_shown() {
        assert_eq!(control_label(TimeControl::new(10, 0)), "10 min");
        assert_eq!(control_label(TimeControl::new(3, 2)), "3 | 2");
        set_current(Language::Greek);
        assert_eq!(control_label(TimeControl::new(10, 0)), "10 λεπτά");
        set_current(Language::English);
    }
}
