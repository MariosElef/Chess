use chess_domain::Square;
use gpui_kit::{App, Hsla, component::ActiveTheme as _, hsla, rgb};

/// The board's own colour roles, one set per theme mode, so no view writes a colour literal.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BoardColors {
    pub light_square: Hsla,
    pub dark_square: Hsla,
    pub selected_square: Hsla,
    /// The dots and rings marking where the picked-up piece can go.
    pub move_hint: Hsla,
}

impl BoardColors {
    pub fn current(cx: &App) -> Self {
        if cx.theme().is_dark() {
            Self::dark_mode()
        } else {
            Self::light_mode()
        }
    }

    pub fn background(&self, square: Square, selected: bool) -> Hsla {
        if selected {
            self.selected_square
        } else {
            self.square(square)
        }
    }

    pub fn square(&self, square: Square) -> Hsla {
        if square.is_light() {
            self.light_square
        } else {
            self.dark_square
        }
    }

    fn light_mode() -> Self {
        Self {
            light_square: rgb(0xe9e5dc).into(),
            dark_square: rgb(0x9fb1c4).into(),
            selected_square: rgb(0xf0d27a).into(),
            move_hint: hsla(0., 0., 0., 0.25),
        }
    }

    fn dark_mode() -> Self {
        Self {
            light_square: rgb(0xb8c1cc).into(),
            dark_square: rgb(0x6f7f93).into(),
            selected_square: rgb(0xc9a94e).into(),
            move_hint: hsla(0., 0., 0., 0.3),
        }
    }
}
