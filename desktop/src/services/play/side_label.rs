use chess_domain::Side;

use crate::i18n::tr;

/// How a side is written for players, in the language being shown: `White`, `Black`.
pub fn side_label(side: Side) -> String {
    match side {
        Side::White => tr!("play.side.white"),
        Side::Black => tr!("play.side.black"),
    }
}

#[cfg(test)]
mod tests {
    use chess_domain::Side;

    use super::side_label;
    use crate::{enums::settings::Language, i18n::set_current};

    #[test]
    fn a_side_is_named_in_the_language_being_shown() {
        assert_eq!(side_label(Side::Black), "Black");
        set_current(Language::Greek);
        assert_eq!(side_label(Side::Black), "Μαύρα");
        set_current(Language::English);
    }
}
