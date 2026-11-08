/// The puzzle ratings a player asks for, both ends included.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RatingRange {
    pub min: i32,
    pub max: i32,
}

impl RatingRange {
    /// The lowest and highest ratings the server serves.
    pub const LOWEST: i32 = 400;
    pub const HIGHEST: i32 = 3200;
    /// Where a new player starts: easy to middling.
    pub const DEFAULT: Self = Self {
        min: 1000,
        max: 1800,
    };

    /// `width` wide around `center`, kept within what the server serves.
    pub fn around(center: i32, width: i32) -> Self {
        let min = (center - width / 2).clamp(Self::LOWEST, Self::HIGHEST);
        Self {
            min,
            max: (min + width).clamp(Self::LOWEST, Self::HIGHEST),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::RatingRange;

    #[test]
    fn a_range_around_a_rating_stays_within_what_is_served() {
        assert_eq!(
            RatingRange::around(1500, 100),
            RatingRange {
                min: 1450,
                max: 1550
            }
        );
        assert_eq!(RatingRange::around(300, 100).min, RatingRange::LOWEST);
        assert_eq!(RatingRange::around(3300, 100).max, RatingRange::HIGHEST);
    }
}
