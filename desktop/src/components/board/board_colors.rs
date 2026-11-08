use chess_domain::Square;
use gpui_kit::{App, Hsla, component::ActiveTheme as _, hsla, rgb, rgba};

use super::board_style::BoardStyle;
use crate::enums::{board::SquareHighlight, settings::BoardTheme};

/// The board's colour roles per theme mode, with the chosen theme's squares.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BoardColors {
    pub light_square: Hsla,
    pub dark_square: Hsla,
    pub selected_square: Hsla,
    /// The two squares of the move that led to the shown position.
    pub last_move_square: Hsla,
    /// The king's square while it is in check.
    pub check_square: Hsla,
    /// A square the player marked with a right-click; softer than check so the two read apart.
    pub marked_square: Hsla,
    /// The squares of a queued premove, chess.com's red, see-through so the square shows.
    pub premove_square: Hsla,
    /// Arrows the player drew with a right-drag.
    pub arrow: Hsla,
    /// The engine's best move, a different colour from the player's own arrows.
    pub engine_arrow: Hsla,
    /// A reviewed move's better choice, in the Best icon's green.
    pub best_arrow: Hsla,
    /// The dots and rings marking where the picked-up piece can go.
    pub move_hint: Hsla,
    /// White's and Black's parts of the analysis eval bar.
    pub eval_white: Hsla,
    pub eval_black: Hsla,
    /// Dims the board behind the promotion picker.
    pub overlay: Hsla,
}

impl BoardColors {
    pub fn current(cx: &App) -> Self {
        let is_dark = cx.theme().is_dark();
        let squares = square_colors(BoardStyle::current(cx).theme, is_dark);
        if is_dark {
            Self::dark_mode(squares)
        } else {
            Self::light_mode(squares)
        }
    }

    pub fn background(&self, square: Square, highlight: SquareHighlight) -> Hsla {
        match highlight {
            SquareHighlight::None => self.square(square),
            SquareHighlight::LastMove => self.last_move_square,
            SquareHighlight::Reviewed(color) => self.square(square).blend(review_tint(color)),
            SquareHighlight::Marked => self.marked_square,
            SquareHighlight::Premove => self.square(square).blend(self.premove_square),
            SquareHighlight::Selected => self.selected_square,
            SquareHighlight::Check => self.check_square,
        }
    }

    pub fn square(&self, square: Square) -> Hsla {
        if square.is_light() {
            self.light_square
        } else {
            self.dark_square
        }
    }

    /// The coordinate letters take the other square colour, so they read on either.
    pub fn coordinate(&self, square: Square) -> Hsla {
        if square.is_light() {
            self.dark_square
        } else {
            self.light_square
        }
    }

    fn light_mode((light_square, dark_square): (Hsla, Hsla)) -> Self {
        Self {
            light_square,
            dark_square,
            selected_square: rgb(0xf0d27a).into(),
            last_move_square: rgb(0xe3d79b).into(),
            check_square: rgb(0xd9483b).into(),
            marked_square: rgb(0xeb8f80).into(),
            premove_square: rgba(0xd9_6c_5f_99).into(),
            arrow: hsla(0.09, 0.95, 0.5, 0.8),
            engine_arrow: hsla(0.6, 0.75, 0.45, 0.75),
            best_arrow: rgba(0x81b6_4ccc).into(),
            move_hint: hsla(0., 0., 0., 0.25),
            eval_white: rgb(0xf4f2ee).into(),
            eval_black: rgb(0x3a3a3c).into(),
            overlay: hsla(0., 0., 0., 0.45),
        }
    }

    fn dark_mode((light_square, dark_square): (Hsla, Hsla)) -> Self {
        Self {
            light_square,
            dark_square,
            selected_square: rgb(0xc9a94e).into(),
            last_move_square: rgb(0xb4a86a).into(),
            check_square: rgb(0xb5342a).into(),
            marked_square: rgb(0xc9776a).into(),
            premove_square: rgba(0xb8_5a_4e_99).into(),
            arrow: hsla(0.09, 0.9, 0.5, 0.8),
            engine_arrow: hsla(0.6, 0.7, 0.55, 0.75),
            best_arrow: rgba(0x81b6_4ccc).into(),
            move_hint: hsla(0., 0., 0., 0.3),
            eval_white: rgb(0xe8e6e1).into(),
            eval_black: rgb(0x1f1f21).into(),
            overlay: hsla(0., 0., 0., 0.55),
        }
    }
}

/// The light and dark squares of `theme`; only Slate has a darker version for dark mode.
pub fn square_colors(theme: BoardTheme, is_dark: bool) -> (Hsla, Hsla) {
    let (light, dark) = match (theme, is_dark) {
        (BoardTheme::Slate, false) => (0xe9e5dc, 0x9fb1c4),
        (BoardTheme::Slate, true) => (0xb8c1cc, 0x6f7f93),
        (BoardTheme::Brown, _) => (0xf0d9b5, 0xb58863),
        (BoardTheme::Blue, _) => (0xeae9d2, 0x4b7399),
        (BoardTheme::Green, _) => (0xeeeed2, 0x769656),
    };
    (rgb(light).into(), rgb(dark).into())
}

/// A review class's colour at half strength, so the square's own shade shows through.
fn review_tint(color: u32) -> Hsla {
    rgba((color << 8) | 0x80).into()
}

#[cfg(test)]
mod tests {
    use gpui_kit::{Hsla, rgb};

    use super::square_colors;
    use crate::enums::settings::BoardTheme;

    fn hex(color: u32) -> Hsla {
        rgb(color).into()
    }

    #[test]
    fn each_theme_has_its_own_squares() {
        assert_eq!(
            square_colors(BoardTheme::Green, false),
            (hex(0xeeeed2), hex(0x769656))
        );
        assert_eq!(
            square_colors(BoardTheme::Brown, true),
            square_colors(BoardTheme::Brown, false),
            "brown looks the same in dark mode"
        );
        assert_ne!(
            square_colors(BoardTheme::Slate, true),
            square_colors(BoardTheme::Slate, false),
            "slate darkens in dark mode"
        );
    }
}
