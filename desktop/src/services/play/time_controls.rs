use chess_domain::{Category, TimeControl};

/// The time controls the new-game menu offers, three per speed.
pub const PRESETS: [TimeControl; 12] = [
    TimeControl::new(1, 0),
    TimeControl::new(1, 1),
    TimeControl::new(2, 1),
    TimeControl::new(3, 0),
    TimeControl::new(3, 2),
    TimeControl::new(5, 0),
    TimeControl::new(10, 0),
    TimeControl::new(10, 5),
    TimeControl::new(15, 10),
    TimeControl::new(30, 0),
    TimeControl::new(30, 20),
    TimeControl::new(60, 0),
];

/// What the menu starts on: ten minutes, a common default for a game at one table.
pub const DEFAULT: Option<TimeControl> = Some(TimeControl::new(10, 0));

/// The presets of one speed, in menu order.
pub fn presets_in(category: Category) -> Vec<TimeControl> {
    PRESETS
        .into_iter()
        .filter(|control| control.category() == category)
        .collect()
}

#[cfg(test)]
mod tests {
    use chess_domain::Category;

    use super::presets_in;

    #[test]
    fn every_speed_offers_three_presets() {
        for category in Category::ALL {
            assert_eq!(presets_in(category).len(), 3, "{}", category.name());
        }
    }
}
